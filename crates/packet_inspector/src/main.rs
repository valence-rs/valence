mod config;
mod context;
mod packet_widget;
mod state;
mod syntax_highlighting;

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};

use anyhow::bail;
use clap::Parser;
use config::ApplicationConfig;
use context::{Context, Packet};
use egui::{Align2, RichText};
use regex::Regex;
use syntax_highlighting::code_view_ui;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tracing_subscriber::filter::LevelFilter;
use valence_protocol::codec::{PacketDecoder, PacketEncoder};
use valence_protocol::packet::c2s::handshake::handshake::NextState;
use valence_protocol::packet::c2s::handshake::HandshakeC2s;
use valence_protocol::packet::c2s::login::{LoginHelloC2s, LoginKeyC2s};
use valence_protocol::packet::c2s::status::{QueryPingC2s, QueryRequestC2s};
use valence_protocol::packet::s2c::login::LoginSuccessS2c;
use valence_protocol::packet::s2c::status::{QueryPongS2c, QueryResponseS2c};
use valence_protocol::packet::{C2sPlayPacket, S2cLoginPacket, S2cPlayPacket};

use crate::context::{ContextMode, Stage};
use crate::packet_widget::PacketDirection;
use crate::state::State;

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about)]
struct Cli {
    /// The socket address to listen for connections on. This is the address
    /// clients should connect to.
    #[arg(required_if_eq("nogui", "true"))]
    client_addr: Option<SocketAddr>,

    /// The socket address the proxy will connect to. This is the address of the
    /// server.
    #[arg(required_if_eq("nogui", "true"))]
    server_addr: Option<SocketAddr>,

    /// The maximum number of connections allowed to the proxy. By default,
    /// there is no limit.
    #[clap(short, long)]
    max_connections: Option<usize>,

    /// Disable the GUI. Logging to stdout.
    #[clap(long)]
    nogui: bool,

    /// Only show packets that match the filter.
    #[clap(short, long)]
    include_filter: Option<Regex>,

    /// Hide packets that match the filter. Note: Only in effect if nogui is
    /// set.
    #[clap(short, long)]
    exclude_filter: Option<Regex>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .init();

    let cli = Arc::new(Cli::parse());

    match cli.nogui {
        true => start_cli(cli).await?,
        false => start_gui(cli)?,
    };

    Ok(())
}

async fn start_cli(cli: Arc<Cli>) -> Result<(), Box<dyn std::error::Error>> {
    let context = Arc::new(Context::new(ContextMode::Cli(context::Logger {
        include_filter: cli.include_filter.clone(),
        exclude_filter: cli.exclude_filter.clone(),
    })));

    let sema = Arc::new(Semaphore::new(cli.max_connections.unwrap_or(100_000)));

    let client_addr = match cli.client_addr {
        Some(addr) => addr,
        None => panic!("client_addr must be set"),
    };

    let server_addr = match cli.server_addr {
        Some(addr) => addr,
        None => panic!("server_addr must be set"),
    };

    eprintln!("Waiting for connections on {}", client_addr);
    let listen = TcpListener::bind(client_addr).await?;

    while let Ok(permit) = sema.clone().acquire_owned().await {
        let (client, remote_client_addr) = listen.accept().await?;
        eprintln!("Accepted connection to {remote_client_addr}");

        if let Err(e) = client.set_nodelay(true) {
            eprintln!("Failed to set TCP_NODELAY: {e}");
        }

        let context = context.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(client, server_addr, context).await {
                eprintln!("Connection to {remote_client_addr} ended with: {e:#}");
            } else {
                eprintln!("Connection to {remote_client_addr} ended.");
            }
            drop(permit);
        });
    }

    Ok(())
}

fn start_gui(cli: Arc<Cli>) -> Result<(), Box<dyn std::error::Error>> {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::Vec2::new(800.0, 600.0)),
        decorated: true,
        ..Default::default()
    };

    let server_addr = cli.server_addr;
    let client_addr = cli.client_addr;
    let max_connections = cli.max_connections.unwrap_or(100_000);

    let filter = cli
        .include_filter
        .clone()
        .map(|f| f.to_string())
        .unwrap_or("".to_string());

    eframe::run_native(
        "Valence Packet Inspector",
        native_options,
        Box::new(move |cc| {
            let gui_app = GuiApp::new(cc, filter);

            if let Some(server_addr) = server_addr {
                if let Some(client_addr) = client_addr {
                    gui_app.start_listening(client_addr, server_addr, max_connections);
                }
            }

            Box::new(gui_app)
        }),
    )?;

    Ok(())
}

async fn handle_connection(
    client: TcpStream,
    server_addr: SocketAddr,
    context: Arc<Context>,
) -> anyhow::Result<()> {
    eprintln!("Connecting to {}", server_addr);

    let server = TcpStream::connect(server_addr).await?;

    if let Err(e) = server.set_nodelay(true) {
        eprintln!("Failed to set TCP_NODELAY: {e}");
    }

    let (client_read, client_write) = client.into_split();
    let (server_read, server_write) = server.into_split();

    let mut s2c = State {
        enc: PacketEncoder::new(),
        dec: PacketDecoder::new(),
        read: server_read,
        write: client_write,
        direction: PacketDirection::ServerToClient,
        context: context.clone(),
    };

    let mut c2s = State {
        enc: PacketEncoder::new(),
        dec: PacketDecoder::new(),
        read: client_read,
        write: server_write,
        direction: PacketDirection::ClientToServer,
        context: context.clone(),
    };

    let handshake: HandshakeC2s = c2s.rw_packet(Stage::HandshakeC2s).await?;

    match handshake.next_state {
        NextState::Status => {
            c2s.rw_packet::<QueryRequestC2s>(Stage::QueryRequestC2s)
                .await?;
            s2c.rw_packet::<QueryResponseS2c>(Stage::QueryResponseS2c)
                .await?;
            c2s.rw_packet::<QueryPingC2s>(Stage::QueryPingC2s).await?;
            s2c.rw_packet::<QueryPongS2c>(Stage::QueryPongS2c).await?;

            Ok(())
        }
        NextState::Login => {
            c2s.rw_packet::<LoginHelloC2s>(Stage::LoginHelloC2s).await?;

            match s2c
                .rw_packet::<S2cLoginPacket>(Stage::S2cLoginPacket)
                .await?
            {
                S2cLoginPacket::LoginHelloS2c(_) => {
                    c2s.rw_packet::<LoginKeyC2s>(Stage::LoginKeyC2s).await?;

                    eprintln!(
                        "Encryption is enabled! Packet contents are inaccessible to the proxy. \
                         Disable online mode to fix this."
                    );

                    context
                        .has_encryption_enabled_error
                        .store(true, Ordering::Relaxed);

                    return tokio::select! {
                        c2s_res = passthrough(c2s.read, c2s.write) => c2s_res,
                        s2c_res = passthrough(s2c.read, s2c.write) => s2c_res,
                    };
                }
                S2cLoginPacket::LoginCompressionS2c(pkt) => {
                    let threshold = pkt.threshold.0 as u32;

                    s2c.enc.set_compression(Some(threshold));
                    s2c.dec.set_compression(true);
                    c2s.enc.set_compression(Some(threshold));
                    c2s.dec.set_compression(true);

                    s2c.rw_packet::<LoginSuccessS2c>(Stage::LoginSuccessS2c)
                        .await?;
                }
                S2cLoginPacket::LoginSuccessS2c(_) => {}
                S2cLoginPacket::LoginDisconnectS2c(_) => return Ok(()),
                S2cLoginPacket::LoginQueryRequestS2c(_) => {
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

#[derive(Clone)]
struct MetaPacket {
    id: i32,
    direction: PacketDirection,
    name: String,
}

impl From<(i32, PacketDirection, String)> for MetaPacket {
    fn from((id, direction, name): (i32, PacketDirection, String)) -> Self {
        Self {
            id,
            direction,
            name,
        }
    }
}

// implement a compare to be used in BTreeMap
impl Ord for MetaPacket {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for MetaPacket {
    fn partial_cmp(&self, other: &Self) -> std::option::Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for MetaPacket {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.direction == other.direction
    }
}

impl Eq for MetaPacket {}

struct GuiApp {
    config: ApplicationConfig,
    temp_server_addr: String,
    temp_client_addr: String,
    temp_max_connections: String,

    server_addr_error: bool,
    client_addr_error: bool,
    max_connections_error: bool,

    context: Arc<Context>,
    filter: String,
    selected_packets: BTreeMap<MetaPacket, bool>,
    buffer: String,
    is_listening: RwLock<bool>,
    window_open: bool,
    encryption_error_dialog_open: bool,
}

impl GuiApp {
    fn new(cc: &eframe::CreationContext<'_>, filter: String) -> Self {
        let ctx = cc.egui_ctx.clone();

        let context = Context::new(ContextMode::Gui(ctx));

        {
            let mut f = context.filter.write().expect("Poisoned filter");
            *f = filter.clone();
        }

        let context = Arc::new(context);

        let config = ApplicationConfig::load();

        let temp_server_addr = config.server_addr().to_string();
        let temp_client_addr = config.client_addr().to_string();
        let temp_max_connections = match config.max_connections() {
            Some(max_connections) => max_connections.to_string(),
            None => String::new(),
        };

        Self {
            config,
            context,
            filter,
            selected_packets: BTreeMap::new(),
            buffer: String::new(),
            is_listening: RwLock::new(false),
            window_open: false,
            encryption_error_dialog_open: false,

            temp_server_addr,
            temp_client_addr,
            temp_max_connections,

            server_addr_error: false,
            client_addr_error: false,
            max_connections_error: false,
        }
    }

    fn start_listening(
        &self,
        client_addr: SocketAddr,
        server_addr: SocketAddr,
        max_connections: usize,
    ) {
        let t_context = self.context.clone();
        tokio::spawn(async move {
            let sema = Arc::new(Semaphore::new(max_connections));

            let listen = TcpListener::bind(client_addr).await?;
            eprintln!("Waiting for connections on {}", client_addr);

            while let Ok(permit) = sema.clone().acquire_owned().await {
                let (client, remote_client_addr) = listen.accept().await?;
                eprintln!("Accepted connection to {remote_client_addr}");

                if let Err(e) = client.set_nodelay(true) {
                    eprintln!("Failed to set TCP_NODELAY: {e}");
                }

                let t2_context = t_context.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(client, server_addr, t2_context).await {
                        eprintln!("Connection to {remote_client_addr} ended with: {e:#}");
                    } else {
                        eprintln!("Connection to {remote_client_addr} ended.");
                    }
                    drop(permit);
                });
            }

            Ok::<(), anyhow::Error>(())
        });
        *self.is_listening.write().expect("Poisoned is_listening") = true;
    }

    fn nested_menus(&mut self, ui: &mut egui::Ui) {
        self.selected_packets
            .iter_mut()
            .for_each(|(m_packet, selected)| {
                // todo: format, add arrows, etc
                if ui.checkbox(selected, m_packet.name.clone()).changed() {
                    ui.ctx().request_repaint();
                }
            });
    }
}

impl eframe::App for GuiApp {
    fn on_close_event(&mut self) -> bool {
        match self.config.save() {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to save config: {e}");
            }
        }
        true
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !*self.is_listening.read().expect("Poisoned is_listening") {
            self.window_open = true;
        }

        if self
            .context
            .has_encryption_enabled_error
            .load(Ordering::Relaxed)
        {
            self.encryption_error_dialog_open = true;
        }

        if self.encryption_error_dialog_open {
            egui::Window::new("Encryption Error")
                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut self.encryption_error_dialog_open)
                .movable(false)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(
                        "Encryption is enabled! Packet contents are inaccessible to the proxy. \
                         Disable online mode to fix this.",
                    );
                });

            // it was true, now it's false, the user acknowledged the error, set it to false
            if !self.encryption_error_dialog_open {
                self.context
                    .has_encryption_enabled_error
                    .store(false, Ordering::Relaxed);
            }
        }

        if self.window_open {
            egui::Window::new("Setup")
                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                .movable(false)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    egui::Grid::new("setup_grid")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Server address:").color(
                                match self.server_addr_error {
                                    true => egui::Color32::RED,
                                    false => egui::Color32::WHITE,
                                },
                            ));
                            if ui
                                .text_edit_singleline(&mut self.temp_server_addr)
                                .on_hover_text(
                                    "The socket address the proxy will connect to. This is the \
                                     address of the server.",
                                )
                                .changed()
                            {
                                self.server_addr_error = false;
                            };
                            ui.end_row();
                            ui.label(RichText::new("Client address:").color(
                                match self.client_addr_error {
                                    true => egui::Color32::RED,
                                    false => egui::Color32::WHITE,
                                },
                            ));
                            ui.text_edit_singleline(&mut self.temp_client_addr)
                                .on_hover_text(
                                    "The socket address to listen for connections on. This is the \
                                     address clients should connect to.",
                                );
                            ui.end_row();
                            ui.label(RichText::new("Max Connections:").color(
                                match self.max_connections_error {
                                    true => egui::Color32::RED,
                                    false => egui::Color32::WHITE,
                                },
                            ));
                            ui.text_edit_singleline(&mut self.temp_max_connections)
                                .on_hover_text(
                                    "The maximum number of connections allowed to the proxy. By \
                                     default, there is no limit.",
                                );
                            ui.end_row();
                            if ui.button("Start Proxy").clicked() {
                                self.window_open = false;
                            }
                        });
                });

            if !self.window_open {
                let server_addr = self.temp_server_addr.parse::<SocketAddr>().map_err(|_| {
                    self.server_addr_error = true;
                });

                let client_addr = self.temp_client_addr.parse::<SocketAddr>().map_err(|_| {
                    self.client_addr_error = true;
                });

                let max_connections = if self.temp_max_connections.is_empty() {
                    Ok(100_000)
                } else {
                    self.temp_max_connections.parse::<usize>().map_err(|_| {
                        self.max_connections_error = true;
                    })
                };

                if server_addr.is_err() || client_addr.is_err() || max_connections.is_err() {
                    self.window_open = true;
                    return;
                }

                self.config.set_server_addr(server_addr.unwrap());
                self.config.set_client_addr(client_addr.unwrap());
                self.config
                    .set_max_connections(if self.temp_max_connections.is_empty() {
                        None
                    } else {
                        Some(self.temp_max_connections.parse::<usize>().unwrap())
                    });

                self.start_listening(
                    client_addr.unwrap(),
                    server_addr.unwrap(),
                    max_connections.unwrap(),
                );
            }

            return;
        }

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Filter:");
                if ui.text_edit_singleline(&mut self.filter).changed() {
                    self.context.set_filter(self.filter.clone());
                }
                ui.menu_button("Packets", |ui| {
                    ui.set_max_width(250.0);
                    ui.set_max_height(400.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink([true, true])
                        .show(ui, |ui| {
                            self.nested_menus(ui);
                        });
                });
            });
        });

        egui::SidePanel::left("side_panel")
            .min_width(150.0)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Packets");

                    let count = self.context.packet_count.read().expect("Poisoned RwLock");
                    let total = self.context.packets.read().expect("Poisoned RwLock").len();

                    let all_selected = self.selected_packets.values().all(|v| *v);

                    if self.filter.is_empty() && all_selected {
                        ui.label(format!("({total})"));
                    } else {
                        ui.label(format!("({count}/{total})"));
                    }

                    if ui.button("Clear").clicked() {
                        self.context.clear();
                        self.buffer = String::new();
                    }

                    if ui.button("Export").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Text Document", &["txt"])
                            .save_file()
                        {
                            match self.context.save(path) {
                                Ok(_) => {}
                                Err(err) => {
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
                        let mut f = self.context.packets.write().expect("Poisoned RwLock");

                        let f: Vec<&mut Packet> = f
                            .iter_mut()
                            .filter(|p| {
                                // what if packets exist (or will exist) with the same name but
                                // different direction?

                                let m_packet = MetaPacket {
                                    id: p.packet_type,
                                    direction: p.direction.clone(),
                                    name: p.packet_name.clone(),
                                };

                                if !self.selected_packets.contains_key(&m_packet) {
                                    self.selected_packets.insert(m_packet.clone(), true);
                                }

                                if let Some(selected) = self.selected_packets.get(&m_packet) {
                                    if !*selected {
                                        return false;
                                    }
                                }

                                if self.filter.is_empty() {
                                    return true;
                                }

                                if let Ok(re) = regex::Regex::new(&self.filter) {
                                    return re.is_match(&p.packet_name);
                                }

                                false
                            })
                            .collect();

                        *self.context.packet_count.write().expect("Poisoned RwLock") = f.len();

                        for packet in f {
                            {
                                let selected = self
                                    .context
                                    .selected_packet
                                    .read()
                                    .expect("Poisoned RwLock");
                                if let Some(idx) = *selected {
                                    if idx == packet.id {
                                        packet.selected(true);
                                        self.buffer = packet.get_packet_string(true);
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
            egui::ScrollArea::vertical().show(ui, |ui| {
                code_view_ui(ui, &self.buffer);
            });
        });
    }
}
