mod context;
mod packet_widget;
mod state;
mod syntax_highlighting;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::bail;
use clap::Parser;
use context::{Context, DisplayPacket};
use syntax_highlighting::code_view_ui;
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
use valence_protocol::{PacketDecoder, PacketEncoder};

use crate::context::Stage;
use crate::packet_widget::PacketDirection;
use crate::state::State;

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about)]
struct Cli {
    /// The socket address to listen for connections on. This is the address
    /// clients should connect to.
    client_addr: SocketAddr,
    /// The socket address the proxy will connect to. This is the address of the
    /// server.
    server_addr: SocketAddr,
    /// The maximum number of connections allowed to the proxy. By default,
    /// there is no limit.
    #[clap(short, long)]
    max_connections: Option<usize>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .init();

    let cli = Arc::new(Cli::parse());

    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::Vec2::new(800.0, 600.0)),
        decorated: true,
        ..Default::default()
    };

    eframe::run_native(
        "Valence Packet Inspector",
        native_options,
        Box::new(|cc| Box::new(App::new(cc, cli))),
    )?;

    Ok(())
}

async fn handle_connection(
    client: TcpStream,
    cli: Arc<Cli>,
    context: Arc<Context>,
) -> anyhow::Result<()> {
    eprintln!("Connecting to {}", cli.server_addr);

    let server = TcpStream::connect(cli.server_addr).await?;

    if let Err(e) = server.set_nodelay(true) {
        eprintln!("Failed to set TCP_NODELAY: {e}");
    }

    let (client_read, client_write) = client.into_split();
    let (server_read, server_write) = server.into_split();

    let mut s2c = State {
        // cli: cli.clone(),
        enc: PacketEncoder::new(),
        dec: PacketDecoder::new(),
        read: server_read,
        write: client_write,
        direction: PacketDirection::ServerToClient,
        context: context.clone(),
    };

    let mut c2s = State {
        // cli,
        enc: PacketEncoder::new(),
        dec: PacketDecoder::new(),
        read: client_read,
        write: server_write,
        direction: PacketDirection::ClientToServer,
        context: context.clone(),
    };

    let handshake: Handshake = c2s.rw_packet(Stage::Handshake).await?;

    match handshake.next_state {
        HandshakeNextState::Status => {
            c2s.rw_packet::<StatusRequest>(Stage::StatusRequest).await?;
            s2c.rw_packet::<StatusResponse>(Stage::StatusResponse)
                .await?;
            c2s.rw_packet::<PingRequest>(Stage::PingRequest).await?;
            s2c.rw_packet::<PingResponse>(Stage::PingResponse).await?;

            Ok(())
        }
        HandshakeNextState::Login => {
            c2s.rw_packet::<LoginStart>(Stage::LoginStart).await?;

            match s2c
                .rw_packet::<S2cLoginPacket>(Stage::S2cLoginPacket)
                .await?
            {
                S2cLoginPacket::EncryptionRequest(_) => {
                    c2s.rw_packet::<EncryptionResponse>(Stage::EncryptionResponse)
                        .await?;

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

                    s2c.rw_packet::<LoginSuccess>(Stage::LoginSuccess).await?;
                }
                S2cLoginPacket::LoginSuccess(_) => {}
                S2cLoginPacket::DisconnectLogin(_) => return Ok(()),
                S2cLoginPacket::LoginPluginRequest(_) => {
                    bail!("got login plugin request. Don't know how to proceed.")
                }
            }

            let c2s_fut: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
                loop {
                    c2s.rw_packet::<C2sPlayPacket>(Stage::C2sPlayPacket).await?;
                }
            });

            let s2c_fut: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
                loop {
                    s2c.rw_packet::<S2cPlayPacket>(Stage::S2cPlayPacket).await?;
                }
            });

            tokio::select! {
                c2s = c2s_fut => Ok(c2s??),
                s2c = s2c_fut => Ok(s2c??),
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

struct App {
    context: Arc<Context>,
    filter: String,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, cli: Arc<Cli>) -> Self {
        let ctx = Some(cc.egui_ctx.clone());
        let context = Arc::new(Context::new(ctx));

        let t_context = context.clone();
        tokio::spawn(async move {
            let sema = Arc::new(Semaphore::new(cli.max_connections.unwrap_or(100_000)));

            eprintln!("Waiting for connections on {}", cli.client_addr);
            let listen = TcpListener::bind(cli.client_addr).await?;

            while let Ok(permit) = sema.clone().acquire_owned().await {
                let (client, remote_client_addr) = listen.accept().await?;
                eprintln!("Accepted connection to {remote_client_addr}");

                if let Err(e) = client.set_nodelay(true) {
                    eprintln!("Failed to set TCP_NODELAY: {e}");
                }

                let t_cli = cli.clone();
                let t2_context = t_context.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(client, t_cli, t2_context).await {
                        eprintln!("Connection to {remote_client_addr} ended with: {e:#}");
                    } else {
                        eprintln!("Connection to {remote_client_addr} ended.");
                    }
                    drop(permit);
                });
            }

            Ok::<(), anyhow::Error>(())
        });

        let t_context = context.clone();
        tokio::spawn(async move {
            loop {
                let packet = t_context
                    .process_packets
                    .write()
                    .expect("Poisoned RwLock")
                    .pop_front();

                if let Some(p) = packet {
                    t_context
                        .packets
                        .write()
                        .expect("Poisoned RwLock")
                        .push(p.into());
                    if let Some(ctx) = &t_context.context {
                        ctx.request_repaint();
                    }
                } else {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        });

        Self {
            context,
            filter: "".into(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Filter:");
                if ui.text_edit_singleline(&mut self.filter).changed() {
                    self.context.set_filter(self.filter.clone());
                }
            });
        });

        egui::SidePanel::left("side_panel")
            .min_width(150.0)
            .default_width(250.0)
            .show(ctx, |ui| {
                // scroll container
                ui.horizontal(|ui| {
                    ui.heading("Packets");

                    let count = self.context.packet_count.read().expect("Poisoned RwLock");
                    let total = self.context.packets.read().expect("Poisoned RwLock").len();

                    if self.filter.is_empty() {
                        ui.label(format!("({total})"));
                    } else {
                        ui.label(format!("({count}/{total})"));

                    }

                    if ui.button("Clear").clicked() {
                        self.context.clear();
                    }

                    if ui.button("Export").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Text Document", &["txt"])
                        .save_file() {
                            match self.context.save(path) {
                                Ok(_) => {},
                                Err(err) => {
                                    // some alert box?
                                    eprintln!("Failed to save: {}", err);
                                }
                            }
                        }
                    }

                });
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {

                        let mut f = self
                            .context
                            .packets
                            .write()
                            .expect("Poisoned RwLock");

                        let f: Vec<&mut DisplayPacket> = f
                            .iter_mut()
                            // todo: regex? or even a wireshark-style filter language processor?
                            .filter(|p| p.packet_name.to_lowercase().contains(&self.filter.to_lowercase()))
                            .collect();

                        *self.context.packet_count.write().expect("Poisoned RwLock") = f.len();

                        for packet in f
                        {
                            {
                                let selected = self
                                    .context
                                    .selected_packet
                                    .read()
                                    .expect("Poisoned RwLock");
                                if let Some(idx) = *selected {
                                    if idx == packet.id {
                                        packet.selected(true);
                                    } else {
                                        packet.selected(false);
                                    }
                                } else {
                                    packet.selected(false);
                                }
                            }

                            if ui.add(packet.clone()).clicked() {
                                self.context.set_selected_packet(packet.id);
                            }
                        }
                    });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(idx) = *self
                .context
                .selected_packet
                .read()
                .expect("Poisoned RwLock")
            {
                // get the packet
                let packets = self.context.packets.read().expect("Poisoned RwLock");
                if idx < packets.len() {
                    let packet = &packets[idx];
                    let text = packet.packet_str.clone();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        code_view_ui(ui, &text);
                    });
                }
            }
        });
    }
}
