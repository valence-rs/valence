use std::error::Error;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::{fmt, io};

use anyhow::bail;
use chrono::{DateTime, Utc};
use clap::Parser;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use valence::protocol::codec::Decoder;
use valence::protocol::packets::handshake::{Handshake, HandshakeNextState};
use valence::protocol::packets::login::c2s::{EncryptionResponse, LoginStart};
use valence::protocol::packets::login::s2c::{LoginSuccess, S2cLoginPacket};
use valence::protocol::packets::c2s::play::C2sPlayPacket;
use valence::protocol::packets::s2c::play::S2cPlayPacket;
use valence::protocol::packets::status::c2s::{QueryPing, QueryRequest};
use valence::protocol::packets::status::s2c::{QueryPong, QueryResponse};
use valence::protocol::packets::{DecodePacket, EncodePacket};
use valence::protocol::{Encode, VarInt};

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about)]
struct Cli {
    /// The socket address to listen for connections on. This is the address
    /// clients should connect to.
    client: SocketAddr,
    /// The socket address the proxy will connect to.
    server: SocketAddr,

    /// The maximum number of connections allowed to the proxy. By default,
    /// there is no limit.
    #[clap(short, long)]
    max_connections: Option<usize>,

    /// Print a timestamp before each packet.
    #[clap(short, long)]
    timestamp: bool,
}

impl Cli {
    fn print(&self, d: &impl fmt::Debug) {
        if self.timestamp {
            let now: DateTime<Utc> = Utc::now();
            println!("{now} {d:?}");
        } else {
            println!("{d:?}");
        }
    }

    async fn rw_packet<P: DecodePacket + EncodePacket>(
        &self,
        read: &mut Decoder<OwnedReadHalf>,
        write: &mut OwnedWriteHalf,
    ) -> anyhow::Result<P> {
        let pkt = read.read_packet().await;

        if let Ok(pkt) = &pkt {
            self.print(pkt);
        }

        let mut len_buf = [0u8; VarInt::MAX_SIZE];
        let len = VarInt(read.packet_buf().len() as i32);
        len.encode(&mut len_buf.as_mut_slice())?;

        write.write_all(&len_buf[..len.written_size()]).await?;
        write.write_all(read.packet_buf()).await?;

        pkt
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let sema = Arc::new(Semaphore::new(
        cli.max_connections.unwrap_or(usize::MAX).min(100_000),
    ));

    eprintln!("Waiting for connections on {}", cli.client);
    let listen = TcpListener::bind(cli.client).await?;

    while let Ok(permit) = sema.clone().acquire_owned().await {
        let (client, remote_client_addr) = listen.accept().await?;
        eprintln!("Accepted connection to {remote_client_addr}");

        let cli = cli.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(client, cli).await {
                eprintln!("Connection to {remote_client_addr} ended with: {e:#}");
            } else {
                eprintln!("Connection to {remote_client_addr} ended.");
            }
            drop(permit);
        });
    }

    Ok(())
}

async fn handle_connection(client: TcpStream, cli: Cli) -> anyhow::Result<()> {
    eprintln!("Connecting to {}", cli.server);

    let server = TcpStream::connect(cli.server).await?;

    let (client_read, mut client_write) = client.into_split();
    let (server_read, mut server_write) = server.into_split();

    let timeout = Duration::from_secs(10);

    let mut client_read = Decoder::new(client_read, timeout);

    let mut server_read = Decoder::new(server_read, timeout);

    let handshake: Handshake = cli.rw_packet(&mut client_read, &mut server_write).await?;

    match handshake.next_state {
        HandshakeNextState::Status => {
            cli.rw_packet::<QueryRequest>(&mut client_read, &mut server_write)
                .await?;
            cli.rw_packet::<QueryResponse>(&mut server_read, &mut client_write)
                .await?;

            cli.rw_packet::<QueryPing>(&mut client_read, &mut server_write)
                .await?;
            cli.rw_packet::<QueryPong>(&mut server_read, &mut client_write)
                .await?;
        }
        HandshakeNextState::Login => {
            cli.rw_packet::<LoginStart>(&mut client_read, &mut server_write)
                .await?;

            match cli
                .rw_packet::<S2cLoginPacket>(&mut server_read, &mut client_write)
                .await?
            {
                S2cLoginPacket::EncryptionRequest(_) => {
                    cli.rw_packet::<EncryptionResponse>(&mut client_read, &mut server_write)
                        .await?;

                    eprintln!("Encryption was enabled! I can't see what's going on anymore.");

                    return tokio::select! {
                        c2s = passthrough(client_read.into_inner(), server_write) => c2s,
                        s2c = passthrough(server_read.into_inner(), client_write) => s2c,
                    };
                }
                S2cLoginPacket::LoginCompression(pkt) => {
                    let threshold = pkt.threshold.0 as u32;
                    client_read.enable_compression(threshold);
                    server_read.enable_compression(threshold);

                    cli.rw_packet::<LoginSuccess>(&mut server_read, &mut client_write)
                        .await?;
                }
                S2cLoginPacket::LoginSuccess(_) => {}
                S2cLoginPacket::LoginDisconnect(_) => return Ok(()),
                S2cLoginPacket::LoginPluginRequest(_) => {
                    bail!("got login plugin request. Don't know how to proceed.")
                }
            }

            let c2s = async {
                loop {
                    if let Err(e) = cli
                        .rw_packet::<C2sPlayPacket>(&mut client_read, &mut server_write)
                        .await
                    {
                        if let Some(e) = e.downcast_ref::<io::Error>() {
                            if e.kind() == ErrorKind::UnexpectedEof {
                                return Ok(());
                            }
                        }
                        eprintln!("Error while decoding serverbound packet: {e:#}");
                    }
                }
            };

            let s2c = async {
                loop {
                    if let Err(e) = cli
                        .rw_packet::<S2cPlayPacket>(&mut server_read, &mut client_write)
                        .await
                    {
                        if let Some(e) = e.downcast_ref::<io::Error>() {
                            if e.kind() == ErrorKind::UnexpectedEof {
                                return Ok(());
                            }
                        }
                        eprintln!("Error while decoding clientbound packet: {e:#}");
                    }
                }
            };

            return tokio::select! {
                c2s = c2s => c2s,
                s2c = s2c => s2c,
            };
        }
    }

    Ok(())
}

async fn passthrough(mut read: OwnedReadHalf, mut write: OwnedWriteHalf) -> anyhow::Result<()> {
    let mut buf = vec![0u8; 4096].into_boxed_slice();
    loop {
        let bytes_read = read.read(&mut buf).await?;
        let bytes = &mut buf[..bytes_read];

        if bytes.is_empty() {
            break;
        }

        write.write_all(bytes).await?;
    }
    Ok(())
}
