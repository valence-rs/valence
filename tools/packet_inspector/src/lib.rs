mod packet_io;
mod packet_registry;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::bail;
use bytes::{BufMut, BytesMut};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use valence_protocol::decode::PacketFrame;
use valence_protocol::packets::handshake::intention_c2s::HandshakeNextState;
use valence_protocol::packets::handshake::IntentionC2s;
use valence_protocol::packets::login::{
    self, HelloS2c, LoginCompressionS2c, LoginDisconnectS2c, LoginFinishedS2c,
};
use valence_protocol::packets::play::LoginS2c;
use valence_protocol::packets::{configuration, play};
use valence_protocol::text::color::NamedColor;
use valence_protocol::text::{Color, IntoText};
use valence_protocol::{
    CompressionThreshold, Decode, Encode, Packet as ValencePacket, PacketSide, PacketState,
};

use crate::packet_io::PacketIo;
pub use crate::packet_registry::Packet;
use crate::packet_registry::PacketRegistry;

include!(concat!(env!("OUT_DIR"), "/packets.rs"));

/// Messages for talking to the running proxy task
#[derive(Debug, Clone)]
pub enum ProxyMessage {
    Stop,
    // In the future there could be a message like
    // InjectPacket(SocketAddr, PacketFrame)
}

#[derive(Debug)]
pub enum DisconnectionReason {
    OnlineModeRequired,
    Error(anyhow::Error),
}

/// Messages sent by the proxy for the controlling GUI/CLI.
#[derive(Debug)]
pub enum ProxyLog {
    /// A new client has connected to the listener.
    ClientConnected(SocketAddr),
    /// A client has disconnected from the listener.
    ClientDisconnected(SocketAddr, DisconnectionReason),
}

pub struct Proxy {
    pub main_task: JoinHandle<anyhow::Result<()>>,
    pub message_tx: flume::Sender<ProxyMessage>,
    pub logs_rx: flume::Receiver<ProxyLog>,
    pub packet_registry: Arc<RwLock<PacketRegistry>>,
}

impl Proxy {
    /// Creates a new proxy, and starts its listener task.
    pub async fn start(listener_addr: SocketAddr, server_addr: SocketAddr) -> anyhow::Result<Self> {
        let (message_tx, message_rx) = flume::unbounded();
        let (logs_tx, logs_rx) = flume::unbounded();

        let packet_registry = Arc::new(RwLock::new({
            let registry = PacketRegistry::new();
            registry.register_all(&STD_PACKETS);
            registry
        }));

        let main_task = tokio::spawn(Self::run_main_task(
            packet_registry.clone(),
            TcpListener::bind(listener_addr).await?,
            server_addr,
            message_rx,
            logs_tx,
        ));

        Ok(Self {
            main_task,
            message_tx,
            logs_rx,
            packet_registry,
        })
    }

    /// Subscribes to the proxy's packet registry.
    pub async fn subscribe(&self) -> flume::Receiver<Packet> {
        self.packet_registry.read().await.subscribe()
    }

    /// Sends a request to stop the proxy and awaits its task's termination.
    /// There's a hardcoded 5 second long timeout after which the task is
    /// considered unresponsive and automatically aborted.
    pub async fn stop(self) {
        // The task may have already stopped, so we can ignore a Disconnected error
        let _ = self.message_tx.send_async(ProxyMessage::Stop).await;

        let abort_handle = self.main_task.abort_handle();
        tokio::select! {
            _ = self.main_task => {},

            // If the main task doesn't stop after 5 seconds, we force terminate it
            () = tokio::time::sleep(Duration::from_secs(5)) => {
                abort_handle.abort();
            },
        }
    }

    /// The main listener task is responsible for handling the TCP listener and
    /// managing child tasks for each client connected to the inspector.
    async fn run_main_task(
        packet_registry: Arc<RwLock<PacketRegistry>>,
        listener: TcpListener,
        server_addr: SocketAddr,
        message_rx: flume::Receiver<ProxyMessage>,
        logs_tx: flume::Sender<ProxyLog>,
    ) -> anyhow::Result<()> {
        let mut individual_tasks = vec![];
        loop {
            tokio::select! {
                r = listener.accept() => {
                    let (stream, addr) = r?;

                    logs_tx.send_async(ProxyLog::ClientConnected(addr)).await?;
                    individual_tasks.push(tokio::spawn(Self::run_individual_proxy(
                        stream,
                        TcpStream::connect(server_addr).await?,
                        logs_tx.clone(),
                        packet_registry.clone(),
                    )));
                }
                m = message_rx.recv_async() => match m {
                    Ok(ProxyMessage::Stop) | Err(_) => {
                        tracing::trace!("Stopping the proxy task...");

                        // TODO: stop these tasks properly instead of just leaving the TCP connections for timeout
                        for task in individual_tasks.drain(..) {
                            task.abort();
                        }

                        return Ok(());
                    }
                }
            }
        }
    }

    /// Each client connected to the inspector is handled in its own individual
    /// task, defined here.
    async fn run_individual_proxy(
        client: TcpStream,
        server: TcpStream,
        a_logs_tx: flume::Sender<ProxyLog>,
        packet_registry: Arc<RwLock<PacketRegistry>>,
    ) -> anyhow::Result<()> {
        let client_addr = client.peer_addr()?;

        let client = PacketIo::new(client);
        let server = PacketIo::new(server);

        let (mut client_reader, mut client_writer) = client.split();
        let (mut server_reader, mut server_writer) = server.split();

        let a_state = Arc::new(RwLock::new(PacketState::Handshake));
        let a_threshold = Arc::new(RwLock::new(CompressionThreshold::DEFAULT));

        let registry = packet_registry.clone();
        let state_lock = a_state.clone();
        let threshold_lock = a_threshold.clone();
        let logs_tx = a_logs_tx.clone();
        let c2s = tokio::spawn(async move {
            loop {
                let threshold = *threshold_lock.read().await;
                client_reader.set_compression(threshold);
                server_writer.set_compression(threshold);

                // client to server handling
                let packet = match client_reader.recv_packet_raw().await {
                    Ok(packet) => packet,
                    Err(e) => {
                        server_writer.shutdown().await?;
                        logs_tx
                            .send_async(ProxyLog::ClientDisconnected(
                                client_addr,
                                DisconnectionReason::Error(e),
                            ))
                            .await?;

                        bail!("connection error");
                    }
                };

                let state = *state_lock.read().await;

                registry
                    .write()
                    .await
                    .process(PacketSide::Serverbound, state, threshold, &packet)
                    .await?;

                if state == PacketState::Handshake {
                    if let Some(handshake) = extrapolate_packet::<IntentionC2s>(&packet) {
                        *state_lock.write().await = match handshake.next_state {
                            HandshakeNextState::Status => PacketState::Status,
                            HandshakeNextState::Login => PacketState::Login,
                        };
                    }
                }
                if state == PacketState::Play {
                    if extrapolate_packet::<play::ConfigurationAcknowledgedC2s>(&packet).is_some() {
                        *state_lock.write().await = PacketState::Configuration;
                    }
                }
                if state == PacketState::Configuration {
                    if extrapolate_packet::<configuration::FinishConfigurationC2s>(&packet)
                        .is_some()
                    {
                        *state_lock.write().await = PacketState::Play;
                    }
                }

                server_writer.send_packet_raw(&packet).await?;
            }
        });

        let registry = packet_registry.clone();
        let state_lock = a_state.clone();
        let threshold_lock = a_threshold.clone();
        let logs_tx = a_logs_tx.clone();
        let s2c = tokio::spawn(async move {
            loop {
                let threshold = *threshold_lock.read().await;
                server_reader.set_compression(threshold);
                client_writer.set_compression(threshold);

                // server to client handling
                let packet = match server_reader.recv_packet_raw().await {
                    Ok(packet) => packet,
                    Err(e) => {
                        client_writer.shutdown().await?;
                        return Err(e);
                    }
                };

                let state = *state_lock.read().await;

                if state == PacketState::Login {
                    if let Some(LoginCompressionS2c { threshold }) = extrapolate_packet(&packet) {
                        *threshold_lock.write().await = CompressionThreshold(threshold.0);
                    }

                    if extrapolate_packet::<LoginFinishedS2c>(&packet).is_some() {
                        *state_lock.write().await = PacketState::Configuration;
                    }
                }

                registry
                    .write()
                    .await
                    .process(PacketSide::Clientbound, state, threshold, &packet)
                    .await?;

                // (The check is done in this if rather than the one above, to still send the
                // encryption request packet to the inspector)
                if state == PacketState::Login && extrapolate_packet::<HelloS2c>(&packet).is_some()
                {
                    // The server is requesting encryption, we can't support that

                    let disconnect_packet = LoginDisconnectS2c {
                        reason: "This server is running in online mode, which is unsupported by \
                                 the Packet Inspector."
                            .into_text()
                            .color(Color::Named(NamedColor::Red))
                            .into_cow_text(),
                    };

                    client_writer
                        .send_packet_raw(&PacketFrame {
                            id: LoginDisconnectS2c::ID,
                            body: {
                                let mut writer = BytesMut::new().writer();
                                disconnect_packet.encode(&mut writer)?;
                                writer.into_inner()
                            },
                        })
                        .await?;

                    client_writer.shutdown().await?;

                    logs_tx
                        .send_async(ProxyLog::ClientDisconnected(
                            client_addr,
                            DisconnectionReason::OnlineModeRequired,
                        ))
                        .await?;

                    bail!("server is running in online mode");
                }

                client_writer.send_packet_raw(&packet).await?;
            }
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
