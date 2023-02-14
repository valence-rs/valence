use std::error::Error;
use std::fmt::Write;
use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::bail;
use clap::Parser;
use owo_colors::OwoColorize;
use regex::Regex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tracing_subscriber::filter::LevelFilter;
use valence_protocol::packets::c2s::handshake::Handshake;
use valence_protocol::packets::c2s::login::{EncryptionResponse, LoginStart};
use valence_protocol::packets::c2s::play::C2sPlayPacket;
use valence_protocol::packets::c2s::status::{PingRequest, StatusRequest};
use valence_protocol::packets::s2c::login::{LoginSuccess, S2cLoginPacket};
use valence_protocol::packets::s2c::play::S2cPlayPacket;
use valence_protocol::packets::s2c::status::{PingResponse, StatusResponse};
use valence_protocol::types::HandshakeNextState;
use valence_protocol::{DecodePacket, EncodePacket, PacketDecoder, PacketEncoder};

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about)]
struct Cli {
    /// The socket address to listen for connections on. This is the address
    /// clients should connect to.
    client_addr: SocketAddr,
    /// The socket address the proxy will connect to. This is the address of the
    /// server.
    server_addr: SocketAddr,
    /// An optional regular expression to use on packet names. Packet names
    /// matching the regex are printed while those that don't are ignored.
    ///
    /// If no regex is provided, all packets are considered matching.
    #[clap(short, long)]
    include_regex: Option<Regex>,
    /// An optional regular expression to use on packet names. Packet names
    /// matching the regex are ignored while those are don't are printed.
    ///
    /// If no regex is provided, all packets are not considered matching.
    #[clap(short, long)]
    exclude_regex: Option<Regex>,
    /// The maximum number of connections allowed to the proxy. By default,
    /// there is no limit.
    #[clap(short, long)]
    max_connections: Option<usize>,
}

struct State {
    cli: Arc<Cli>,
    enc: PacketEncoder,
    dec: PacketDecoder,
    read: OwnedReadHalf,
    write: OwnedWriteHalf,
    buf: String,
    style: owo_colors::Style,
}

impl State {
    pub async fn rw_packet<'a, P>(&'a mut self) -> anyhow::Result<P>
    where
        P: DecodePacket<'a> + EncodePacket,
    {
        while !self.dec.has_next_packet()? {
            self.dec.reserve(4096);
            let mut buf = self.dec.take_capacity();

            if self.read.read_buf(&mut buf).await? == 0 {
                return Err(io::Error::from(ErrorKind::UnexpectedEof).into());
            }

            self.dec.queue_bytes(buf);
        }

        let pkt: P = self.dec.try_next_packet()?.unwrap();

        self.enc.append_packet(&pkt)?;

        let bytes = self.enc.take();
        self.write.write_all(&bytes).await?;

        self.buf.clear();
        write!(&mut self.buf, "{pkt:?}")?;

        let packet_name = self
            .buf
            .split_once(|ch: char| !ch.is_ascii_alphabetic())
            .map(|(fst, _)| fst)
            .unwrap_or(&self.buf);

        if let Some(r) = &self.cli.include_regex {
            if !r.is_match(packet_name) {
                return Ok(pkt);
            }
        }

        if let Some(r) = &self.cli.exclude_regex {
            if r.is_match(packet_name) {
                return Ok(pkt);
            }
        }

        println!("{}", self.buf.style(self.style));

        Ok(pkt)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .init();

    let cli = Arc::new(Cli::parse());

    let sema = Arc::new(Semaphore::new(cli.max_connections.unwrap_or(100_000)));

    eprintln!("Waiting for connections on {}", cli.client_addr);
    let listen = TcpListener::bind(cli.client_addr).await?;

    while let Ok(permit) = sema.clone().acquire_owned().await {
        let (client, remote_client_addr) = listen.accept().await?;
        eprintln!("Accepted connection to {remote_client_addr}");

        if let Err(e) = client.set_nodelay(true) {
            eprintln!("Failed to set TCP_NODELAY: {e}");
        }

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

async fn handle_connection(client: TcpStream, cli: Arc<Cli>) -> anyhow::Result<()> {
    eprintln!("Connecting to {}", cli.server_addr);

    let server = TcpStream::connect(cli.server_addr).await?;

    if let Err(e) = server.set_nodelay(true) {
        eprintln!("Failed to set TCP_NODELAY: {e}");
    }

    let (client_read, client_write) = client.into_split();
    let (server_read, server_write) = server.into_split();

    let mut s2c = State {
        cli: cli.clone(),
        enc: PacketEncoder::new(),
        dec: PacketDecoder::new(),
        read: server_read,
        write: client_write,
        buf: String::new(),
        style: owo_colors::Style::new().purple(),
    };

    let mut c2s = State {
        cli,
        enc: PacketEncoder::new(),
        dec: PacketDecoder::new(),
        read: client_read,
        write: server_write,
        buf: String::new(),
        style: owo_colors::Style::new().green(),
    };

    let handshake: Handshake = c2s.rw_packet().await?;

    match handshake.next_state {
        HandshakeNextState::Status => {
            c2s.rw_packet::<StatusRequest>().await?;
            s2c.rw_packet::<StatusResponse>().await?;
            c2s.rw_packet::<PingRequest>().await?;
            s2c.rw_packet::<PingResponse>().await?;

            Ok(())
        }
        HandshakeNextState::Login => {
            c2s.rw_packet::<LoginStart>().await?;

            match s2c.rw_packet::<S2cLoginPacket>().await? {
                S2cLoginPacket::EncryptionRequest(_) => {
                    c2s.rw_packet::<EncryptionResponse>().await?;

                    eprintln!(
                        "Encryption was enabled! Packet contents are inaccessible to the proxy. \
                         Disable online_mode to fix this."
                    );

                    return tokio::select! {
                        c2s_res = passthrough(c2s.read, c2s.write) => c2s_res,
                        s2c_res = passthrough(s2c.read, s2c.write) => s2c_res,
                    };
                }
                S2cLoginPacket::SetCompression(pkt) => {
                    let threshold = pkt.threshold.0 as u32;

                    s2c.enc.set_compression(Some(threshold));
                    s2c.dec.set_compression(true);
                    c2s.enc.set_compression(Some(threshold));
                    c2s.dec.set_compression(true);

                    s2c.rw_packet::<LoginSuccess>().await?;
                }
                S2cLoginPacket::LoginSuccess(_) => {}
                S2cLoginPacket::DisconnectLogin(_) => return Ok(()),
                S2cLoginPacket::LoginPluginRequest(_) => {
                    bail!("got login plugin request. Don't know how to proceed.")
                }
            }

            let c2s_fut: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
                loop {
                    c2s.rw_packet::<C2sPlayPacket>().await?;
                }
            });

            let s2c_fut = async move {
                loop {
                    s2c.rw_packet::<S2cPlayPacket>().await?;
                }
            };

            tokio::select! {
                c2s = c2s_fut => Ok(c2s??),
                s2c = s2c_fut => s2c,
            }
        }
    }
}

async fn passthrough(mut read: OwnedReadHalf, mut write: OwnedWriteHalf) -> anyhow::Result<()> {
    let mut buf = Box::new([0u8; 8192]);
    loop {
        let bytes_read = read.read(buf.as_mut_slice()).await?;
        let bytes = &mut buf[..bytes_read];

        if bytes.is_empty() {
            break Ok(());
        }

        write.write_all(bytes).await?;
    }
}
