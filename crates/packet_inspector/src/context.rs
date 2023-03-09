use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::RwLock;

use owo_colors::{OwoColorize, Style};
use regex::Regex;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use valence_protocol::codec::PacketDecoder;
use valence_protocol::packet::c2s::handshake::HandshakeC2s;
use valence_protocol::packet::c2s::login::{LoginHelloC2s, LoginKeyC2s};
use valence_protocol::packet::c2s::status::{QueryPingC2s, QueryRequestC2s};
use valence_protocol::packet::s2c::login::LoginSuccessS2c;
use valence_protocol::packet::s2c::status::{QueryPongS2c, QueryResponseS2c};
use valence_protocol::packet::{C2sPlayPacket, S2cLoginPacket, S2cPlayPacket};
use valence_protocol::raw::RawPacket;

use crate::packet_widget::{systemtime_strftime, PacketDirection};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    HandshakeC2s,
    QueryRequestC2s,
    QueryResponseS2c,
    QueryPingC2s,
    QueryPongS2c,
    LoginHelloC2s,
    S2cLoginPacket,
    LoginKeyC2s,
    LoginSuccessS2c,
    C2sPlayPacket,
    S2cPlayPacket,
}

impl From<Stage> for usize {
    fn from(stage: Stage) -> Self {
        match stage {
            Stage::HandshakeC2s => 0,
            Stage::QueryRequestC2s => 1,
            Stage::QueryResponseS2c => 2,
            Stage::QueryPingC2s => 3,
            Stage::QueryPongS2c => 4,
            Stage::LoginHelloC2s => 5,
            Stage::S2cLoginPacket => 6,
            Stage::LoginKeyC2s => 7,
            Stage::LoginSuccessS2c => 8,
            Stage::C2sPlayPacket => 9,
            Stage::S2cPlayPacket => 10,
        }
    }
}

impl TryFrom<usize> for Stage {
    type Error = anyhow::Error;

    fn try_from(value: usize) -> anyhow::Result<Self> {
        match value {
            0 => Ok(Stage::HandshakeC2s),
            1 => Ok(Stage::QueryRequestC2s),
            2 => Ok(Stage::QueryResponseS2c),
            3 => Ok(Stage::QueryPingC2s),
            4 => Ok(Stage::QueryPongS2c),
            5 => Ok(Stage::LoginHelloC2s),
            6 => Ok(Stage::S2cLoginPacket),
            7 => Ok(Stage::LoginKeyC2s),
            8 => Ok(Stage::LoginSuccessS2c),
            9 => Ok(Stage::C2sPlayPacket),
            10 => Ok(Stage::S2cPlayPacket),
            _ => Err(anyhow::anyhow!("Invalid stage")),
        }
    }
}

#[derive(Clone)]
pub struct Packet {
    pub(crate) id: usize,
    pub(crate) direction: PacketDirection,
    pub(crate) selected: bool,
    pub(crate) use_compression: bool,
    pub(crate) packet_data: Vec<u8>,
    pub(crate) stage: Stage,
    pub(crate) packet_type: i32,
    pub(crate) packet_name: String,
    pub(crate) created_at: OffsetDateTime,
}

impl Packet {
    pub(crate) fn selected(&mut self, value: bool) {
        self.selected = value;
    }

    pub fn get_raw_packet(&self) -> Vec<u8> {
        let mut dec = PacketDecoder::new();
        dec.set_compression(self.use_compression);
        dec.queue_slice(&self.packet_data);

        let pkt = match dec.try_next_packet::<RawPacket>() {
            Ok(Some(pkt)) => pkt,
            Ok(None) => return vec![],
            Err(e) => {
                eprintln!("Error decoding packet: {e}");
                return vec![];
            }
        };

        pkt.0.to_vec()
    }

    pub fn get_packet_string(&self, formatted: bool) -> String {
        let mut dec = PacketDecoder::new();
        dec.set_compression(self.use_compression);
        dec.queue_slice(&self.packet_data);

        match self.stage {
            Stage::HandshakeC2s => {
                let pkt = match dec.try_next_packet::<HandshakeC2s>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "HandshakeC2s".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                if formatted {
                    format!("{pkt:#?}")
                } else {
                    format!("{pkt:?}")
                }
            }
            Stage::QueryRequestC2s => {
                let pkt = match dec.try_next_packet::<QueryRequestC2s>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "QueryRequestC2s".to_string(),
                    Err(err) => return format!("{:?}", err),
                };

                if formatted {
                    format!("{pkt:#?}")
                } else {
                    format!("{pkt:?}")
                }
            }
            Stage::QueryResponseS2c => {
                let pkt = match dec.try_next_packet::<QueryResponseS2c>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "QueryResponseS2c".to_string(),
                    Err(err) => return format!("{:?}", err),
                };

                if formatted {
                    format!("{pkt:#?}")
                } else {
                    format!("{pkt:?}")
                }
            }
            Stage::QueryPingC2s => {
                let pkt = match dec.try_next_packet::<QueryPingC2s>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "QueryPingC2s".to_string(),
                    Err(err) => return format!("{:?}", err),
                };

                if formatted {
                    format!("{pkt:#?}")
                } else {
                    format!("{pkt:?}")
                }
            }
            Stage::QueryPongS2c => {
                let pkt = match dec.try_next_packet::<QueryPongS2c>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "QueryPongS2c".to_string(),
                    Err(err) => return format!("{:?}", err),
                };

                if formatted {
                    format!("{pkt:#?}")
                } else {
                    format!("{pkt:?}")
                }
            }
            Stage::LoginHelloC2s => {
                let pkt = match dec.try_next_packet::<LoginHelloC2s>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "LoginHelloC2s".to_string(),
                    Err(err) => return format!("{:?}", err),
                };

                if formatted {
                    format!("{pkt:#?}")
                } else {
                    format!("{pkt:?}")
                }
            }
            Stage::S2cLoginPacket => {
                let pkt = match dec.try_next_packet::<S2cLoginPacket>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "S2cLoginPacket".to_string(),
                    Err(err) => return format!("{:?}", err),
                };

                if formatted {
                    format!("{pkt:#?}")
                } else {
                    format!("{pkt:?}")
                }
            }
            Stage::LoginKeyC2s => {
                let pkt = match dec.try_next_packet::<LoginKeyC2s>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "LoginKeyC2s".to_string(),
                    Err(err) => return format!("{:?}", err),
                };

                if formatted {
                    format!("{pkt:#?}")
                } else {
                    format!("{pkt:?}")
                }
            }
            Stage::LoginSuccessS2c => {
                let pkt = match dec.try_next_packet::<LoginSuccessS2c>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "LoginSuccessS2c".to_string(),
                    Err(err) => return format!("{:?}", err),
                };

                if formatted {
                    format!("{pkt:#?}")
                } else {
                    format!("{pkt:?}")
                }
            }
            Stage::C2sPlayPacket => {
                let pkt = match dec.try_next_packet::<C2sPlayPacket>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "C2sPlayPacket".to_string(),
                    Err(err) => return format!("{:?}", err),
                };

                if formatted {
                    format!("{pkt:#?}")
                } else {
                    format!("{pkt:?}")
                }
            }
            Stage::S2cPlayPacket => {
                let pkt = match dec.try_next_packet::<S2cPlayPacket>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "S2cPlayPacket".to_string(),
                    Err(err) => return format!("{:?}", err),
                };

                if formatted {
                    format!("{pkt:#?}")
                } else {
                    format!("{pkt:?}")
                }
            }
        }
    }
}

pub struct Logger {
    pub include_filter: Option<Regex>,
    pub exclude_filter: Option<Regex>,
}

pub enum ContextMode {
    Gui(egui::Context),
    Cli(Logger),
}

pub struct Context {
    pub mode: ContextMode,
    pub last_packet: AtomicUsize,
    pub selected_packet: RwLock<Option<usize>>,
    pub(crate) packets: RwLock<Vec<Packet>>,
    pub(crate) packet_count: RwLock<usize>,
    pub(crate) has_encryption_enabled_error: AtomicBool,
    pub filter: RwLock<String>,
    c2s_style: Style,
    s2c_style: Style,
}

impl Context {
    pub fn new(mode: ContextMode) -> Self {
        Self {
            mode,
            last_packet: AtomicUsize::new(0),
            selected_packet: RwLock::new(None),
            packets: RwLock::new(Vec::new()),
            filter: RwLock::new("".into()),
            packet_count: RwLock::new(0),

            has_encryption_enabled_error: AtomicBool::new(false),

            c2s_style: Style::new().green(),
            s2c_style: Style::new().purple(),
        }
    }

    pub fn clear(&self) {
        self.last_packet.store(0, Ordering::Relaxed);
        *self.selected_packet.write().expect("Poisoned RwLock") = None;
        self.packets.write().expect("Poisoned RwLock").clear();
        if let ContextMode::Gui(ctx) = &self.mode {
            ctx.request_repaint();
        }
    }

    pub fn add(&self, mut packet: Packet) {
        match &self.mode {
            ContextMode::Gui(ctx) => {
                packet.id = self.last_packet.fetch_add(1, Ordering::Relaxed);
                self.packets.write().expect("Poisoned RwLock").push(packet);
                ctx.request_repaint();
            }
            ContextMode::Cli(logger) => {
                if let Some(include_filter) = &logger.include_filter {
                    if !include_filter.is_match(&packet.packet_name) {
                        return;
                    }
                }
                if let Some(exclude_filter) = &logger.exclude_filter {
                    if exclude_filter.is_match(&packet.packet_name) {
                        return;
                    }
                }

                let arrow = match &packet.direction {
                    PacketDirection::ClientToServer => "↑",
                    PacketDirection::ServerToClient => "↓",
                };

                if atty::is(atty::Stream::Stdout) {
                    let style = match &packet.direction {
                        PacketDirection::ClientToServer => self.c2s_style,
                        PacketDirection::ServerToClient => self.s2c_style,
                    };

                    println!(
                        "[{}] ({}) {}",
                        systemtime_strftime(packet.created_at),
                        arrow.style(style),
                        packet.get_packet_string(false).style(style)
                    );
                } else {
                    println!(
                        "[{}] ({}) {}",
                        systemtime_strftime(packet.created_at),
                        arrow,
                        packet.get_packet_string(false)
                    );
                }
            }
        }
    }

    pub fn set_selected_packet(&self, idx: usize) {
        *self.selected_packet.write().expect("Poisoned RwLock") = Some(idx);
    }

    pub fn set_filter(&self, filter: String) {
        *self.filter.write().expect("Posisoned RwLock") = filter;
        *self.selected_packet.write().expect("Poisoned RwLock") = None;
    }

    pub fn save(&self, path: PathBuf) -> Result<(), std::io::Error> {
        let packets = self
            .packets
            .read()
            .expect("Poisoned RwLock")
            .iter()
            .map(|packet| {
                format!(
                    "[{}] {}",
                    systemtime_strftime(packet.created_at),
                    packet.get_packet_string(false)
                )
            })
            .collect::<Vec<String>>()
            .join("\n");

        std::fs::write(path, packets)?;

        Ok(())
    }
}
