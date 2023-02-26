use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;

use time::OffsetDateTime;
use valence_protocol::codec::PacketDecoder;
use valence_protocol::packet::c2s::handshake::HandshakeC2s;
use valence_protocol::packet::c2s::login::{LoginHelloC2s, LoginKeyC2s};
use valence_protocol::packet::c2s::status::{QueryPingC2s, QueryRequestC2s};
use valence_protocol::packet::s2c::login::LoginSuccessS2c;
use valence_protocol::packet::s2c::status::{QueryPongS2c, QueryResponseS2c};
use valence_protocol::packet::{C2sPlayPacket, S2cLoginPacket, S2cPlayPacket};

use crate::packet_widget::{systemtime_strftime, PacketDirection};

#[derive(Clone)]
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

impl From<&mut Packet> for String {
    fn from(value: &mut Packet) -> Self {
        if value.packet_data.len() > 1024 {
            return "Packet too large".to_string();
        }
        value.get_packet_string()
    }
}

impl Packet {
    pub(crate) fn selected(&mut self, value: bool) {
        self.selected = value;
    }

    fn get_packet_string_no_format(&self) -> String {
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
                format!("{pkt:?}")
            }
            Stage::QueryRequestC2s => {
                let pkt = match dec.try_next_packet::<QueryRequestC2s>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "QueryRequestC2s".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:?}")
            }
            Stage::QueryResponseS2c => {
                let pkt = match dec.try_next_packet::<QueryResponseS2c>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "QueryResponseS2c".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:?}")
            }
            Stage::QueryPingC2s => {
                let pkt = match dec.try_next_packet::<QueryPingC2s>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "QueryPingC2s".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:?}")
            }
            Stage::QueryPongS2c => {
                let pkt = match dec.try_next_packet::<QueryPongS2c>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "QueryPongS2c".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:?}")
            }
            Stage::LoginHelloC2s => {
                let pkt = match dec.try_next_packet::<LoginHelloC2s>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "LoginHelloC2s".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:?}")
            }
            Stage::S2cLoginPacket => {
                let pkt = match dec.try_next_packet::<S2cLoginPacket>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "S2cLoginPacket".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:?}")
            }
            Stage::LoginKeyC2s => {
                let pkt = match dec.try_next_packet::<LoginKeyC2s>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "LoginKeyC2s".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:?}")
            }
            Stage::LoginSuccessS2c => {
                let pkt = match dec.try_next_packet::<LoginSuccessS2c>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "LoginSuccessS2c".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:?}")
            }
            Stage::C2sPlayPacket => {
                let pkt = match dec.try_next_packet::<C2sPlayPacket>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "C2sPlayPacket".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:?}")
            }
            Stage::S2cPlayPacket => {
                let pkt = match dec.try_next_packet::<S2cPlayPacket>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "S2cPlayPacket".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:?}")
            }
        }
    }

    fn get_packet_string(&self) -> String {
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
                format!("{pkt:#?}")
            }
            Stage::QueryRequestC2s => {
                let pkt = match dec.try_next_packet::<QueryRequestC2s>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "QueryRequestC2s".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:#?}")
            }
            Stage::QueryResponseS2c => {
                let pkt = match dec.try_next_packet::<QueryResponseS2c>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "QueryResponseS2c".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:#?}")
            }
            Stage::QueryPingC2s => {
                let pkt = match dec.try_next_packet::<QueryPingC2s>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "QueryPingC2s".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:#?}")
            }
            Stage::QueryPongS2c => {
                let pkt = match dec.try_next_packet::<QueryPongS2c>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "QueryPongS2c".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:#?}")
            }
            Stage::LoginHelloC2s => {
                let pkt = match dec.try_next_packet::<LoginHelloC2s>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "LoginHelloC2s".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:#?}")
            }
            Stage::S2cLoginPacket => {
                let pkt = match dec.try_next_packet::<S2cLoginPacket>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "S2cLoginPacket".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:#?}")
            }
            Stage::LoginKeyC2s => {
                let pkt = match dec.try_next_packet::<LoginKeyC2s>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "LoginKeyC2s".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:#?}")
            }
            Stage::LoginSuccessS2c => {
                let pkt = match dec.try_next_packet::<LoginSuccessS2c>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "LoginSuccessS2c".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:#?}")
            }
            Stage::C2sPlayPacket => {
                let pkt = match dec.try_next_packet::<C2sPlayPacket>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "C2sPlayPacket".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:#?}")
            }
            Stage::S2cPlayPacket => {
                let pkt = match dec.try_next_packet::<S2cPlayPacket>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "S2cPlayPacket".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:#?}")
            }
        }
    }
}

pub struct Context {
    pub last_packet: AtomicUsize,
    pub selected_packet: RwLock<Option<usize>>,
    pub(crate) packets: RwLock<Vec<Packet>>,
    pub(crate) packet_count: RwLock<usize>,
    pub filter: RwLock<String>,
    pub(crate) context: Option<egui::Context>,
}

impl Context {
    pub fn new(ctx: Option<egui::Context>) -> Self {
        Self {
            last_packet: AtomicUsize::new(0),
            selected_packet: RwLock::new(None),
            packets: RwLock::new(Vec::new()),
            filter: RwLock::new("".into()),
            context: ctx,
            packet_count: RwLock::new(0),
        }
    }

    pub fn clear(&self) {
        self.last_packet.store(0, Ordering::Relaxed);
        *self.selected_packet.write().expect("Poisoned RwLock") = None;
        self.packets.write().expect("Poisoned RwLock").clear();
        if let Some(ctx) = &self.context {
            ctx.request_repaint();
        }
    }

    pub fn add(&self, mut packet: Packet) {
        packet.id = self.last_packet.fetch_add(1, Ordering::Relaxed);
        self.packets.write().expect("Poisoned RwLock").push(packet);
        if let Some(ctx) = &self.context {
            ctx.request_repaint();
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
            .filter(|packet| packet.packet_name != "ChunkDataAndUpdateLight") // temporarily blacklisting this packet because HUGE
            .map(|packet| {
                format!("[{}] {}", systemtime_strftime(packet.created_at), packet.get_packet_string_no_format())
            })
            .collect::<Vec<String>>()
            .join("\n");

        std::fs::write(path, packets)?;

        Ok(())
    }
}
