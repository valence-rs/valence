mod packet_io;
mod packet_registry;

use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};

pub use packet_registry::Packet;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use valence::protocol::decode::PacketFrame;
use valence::protocol::packets::handshaking::handshake_c2s::HandshakeNextState;
use valence::protocol::packets::handshaking::HandshakeC2s;
use valence::protocol::packets::login::{LoginCompressionS2c, LoginSuccessS2c};
use valence::protocol::{Decode, Packet as ValencePacket};
use valence::CompressionThreshold;

use crate::packet_io::PacketIo;
use crate::packet_registry::PacketRegistry;
pub use crate::packet_registry::{PacketSide, PacketState};

static PACKET_REGISTRY: OnceLock<Arc<RwLock<PacketRegistry>>> = OnceLock::new();

include!(concat!(env!("OUT_DIR"), "/packets.rs"));

pub struct Proxy {
    listener_addr: SocketAddr,
    server_addr: SocketAddr,
}

impl Proxy {
    pub fn new(listener_addr: SocketAddr, server_addr: SocketAddr) -> Self {
        PACKET_REGISTRY.get_or_init(|| {
            let registry = PacketRegistry::new();
            registry.register_all(&STD_PACKETS);
            Arc::new(RwLock::new(registry))
        });

        Proxy {
            listener_addr,
            server_addr,
        }
    }

    pub async fn subscribe(&self) -> flume::Receiver<Packet> {
        PACKET_REGISTRY.get().unwrap().read().await.subscribe()
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(self.listener_addr).await?;

        while let Ok((client, _addr)) = listener.accept().await {
            let server_addr = self.server_addr;
            tokio::spawn(async move {
                let server = TcpStream::connect(server_addr).await?;

                if let Err(e) = Self::process(client, server).await {
                    if !e.to_string().contains("unexpected end of file") {
                        // bit meh to do it like this but it works
                        tracing::error!("Error: {:?}", e);
                    }
                }

                Ok::<(), anyhow::Error>(())
            });
        }
        Ok(())
    }

    async fn process(client: TcpStream, server: TcpStream) -> anyhow::Result<()> {
        let client = PacketIo::new(client);
        let server = PacketIo::new(server);

        let (mut client_reader, mut client_writer) = client.split();
        let (mut server_reader, mut server_writer) = server.split();

        let current_state_inner = Arc::new(RwLock::new(PacketState::Handshaking));
        let threshold_inner: Arc<RwLock<CompressionThreshold>> = Arc::new(RwLock::new(None));

        let current_state = current_state_inner.clone();
        let threshold = threshold_inner.clone();
        let registry = PACKET_REGISTRY.get().unwrap().clone();
        let c2s = tokio::spawn(async move {
            loop {
                let threshold = *threshold.read().await;
                client_reader.set_compression(threshold);
                server_writer.set_compression(threshold);
                // client to server handling
                let packet = client_reader.recv_packet_raw().await?;

                let state = {
                    let state = current_state.read().await;
                    *state
                };

                let registry = registry.write().await;
                registry
                    .process(
                        crate::packet_registry::PacketSide::Serverbound,
                        state,
                        threshold,
                        &packet,
                    )
                    .await?;

                if state == PacketState::Handshaking {
                    if let Some(handshake) = extrapolate_packet::<HandshakeC2s>(&packet) {
                        *current_state.write().await = match handshake.next_state {
                            HandshakeNextState::Status => PacketState::Status,
                            HandshakeNextState::Login => PacketState::Login,
                        };
                    }
                }

                server_writer.send_packet_raw(&packet).await?;
            }

            #[allow(unreachable_code)]
            Ok::<(), anyhow::Error>(())
        });

        let current_state = current_state_inner.clone();
        let threshold = threshold_inner.clone();
        let registry = PACKET_REGISTRY.get().unwrap().clone();
        let s2c = tokio::spawn(async move {
            loop {
                let threshold_value = *threshold.read().await;
                server_reader.set_compression(threshold_value);
                client_writer.set_compression(threshold_value);
                // server to client handling
                let packet = server_reader.recv_packet_raw().await?;

                let state = {
                    let state = current_state.read().await;
                    *state
                };

                if state == PacketState::Login {
                    if let Some(compression) = extrapolate_packet::<LoginCompressionS2c>(&packet) {
                        if compression.threshold.0 >= 0 {
                            *threshold.write().await = Some(compression.threshold.0 as u32);
                        }
                    };

                    if extrapolate_packet::<LoginSuccessS2c>(&packet).is_some() {
                        *current_state.write().await = PacketState::Play;
                    };
                }

                registry
                    .write()
                    .await
                    .process(
                        crate::packet_registry::PacketSide::Clientbound,
                        state,
                        threshold_value,
                        &packet,
                    )
                    .await?;

                client_writer.send_packet_raw(&packet).await?;
            }

            #[allow(unreachable_code)]
            Ok::<(), anyhow::Error>(())
        });

        // wait for either to finish
        tokio::select! {
            res = c2s => res?,
            res = s2c => res?,
        }
    }
}

fn extrapolate_packet<'a, P>(packet: &'a PacketFrame) -> Option<P>
where
    P: ValencePacket + Decode<'a> + Clone,
{
    if packet.id != P::ID {
        return None;
    }

    let mut r = &packet.body[..];
    let packet = P::decode(&mut r).ok()?;
    Some(packet)
}
