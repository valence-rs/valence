use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;

use time::OffsetDateTime;
use valence_protocol::packets::c2s::handshake::Handshake;
use valence_protocol::packets::c2s::login::{EncryptionResponse, LoginStart};
use valence_protocol::packets::c2s::status::{PingRequest, StatusRequest};
use valence_protocol::packets::s2c::login::LoginSuccess;
use valence_protocol::packets::s2c::status::{PingResponse, StatusResponse};
use valence_protocol::packets::{C2sPlayPacket, S2cLoginPacket, S2cPlayPacket};
use valence_protocol::PacketDecoder;

use crate::packet_widget::PacketDirection;

#[derive(Clone)]
pub enum Stage {
    Handshake,
    StatusRequest,
    StatusResponse,
    PingRequest,
    PingResponse,
    LoginStart,
    S2cLoginPacket,
    EncryptionResponse,
    LoginSuccess,
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
    pub(crate) packet_type: u8,
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

    fn get_packet_string(&self) -> String {
        let mut dec = PacketDecoder::new();
        dec.set_compression(self.use_compression);
        dec.queue_slice(&self.packet_data);

        match self.stage {
            Stage::Handshake => {
                let pkt = match dec.try_next_packet::<Handshake>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "Handshake".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:#?}")
            }
            Stage::StatusRequest => {
                let pkt = match dec.try_next_packet::<StatusRequest>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "StatusRequest".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:#?}")
            }
            Stage::StatusResponse => {
                let pkt = match dec.try_next_packet::<StatusResponse>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "StatusResponse".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:#?}")
            }
            Stage::PingRequest => {
                let pkt = match dec.try_next_packet::<PingRequest>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "PingRequest".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:#?}")
            }
            Stage::PingResponse => {
                let pkt = match dec.try_next_packet::<PingResponse>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "PingResponse".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:#?}")
            }
            Stage::LoginStart => {
                let pkt = match dec.try_next_packet::<LoginStart>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "LoginStart".to_string(),
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
            Stage::EncryptionResponse => {
                let pkt = match dec.try_next_packet::<EncryptionResponse>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "EncryptionResponse".to_string(),
                    Err(err) => return format!("{:?}", err),
                };
                format!("{pkt:#?}")
            }
            Stage::LoginSuccess => {
                let pkt = match dec.try_next_packet::<LoginSuccess>() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => return "LoginSuccess".to_string(),
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
    pub buffer: RwLock<String>,
    pub(crate) context: Option<egui::Context>,
}

impl Context {
    pub fn new(ctx: Option<egui::Context>) -> Self {
        Self {
            last_packet: AtomicUsize::new(0),
            selected_packet: RwLock::new(None),
            packets: RwLock::new(Vec::new()),
            filter: RwLock::new("".into()),
            buffer: RwLock::new("".into()),
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

    // Might want to do this "one at a time..."?
    pub fn save(&self, path: PathBuf) -> Result<(), std::io::Error> {
        let packets = self
            .packets
            .read()
            .expect("Poisoned RwLock")
            .iter()
            .filter(|packet| packet.packet_name != "ChunkDataAndUpdateLight") // temporarily blacklisting this packet because HUGE
            .map(|packet| packet.get_packet_string().replace("    ", "").replace('\n', " ")) // deformat the packet
            .collect::<Vec<String>>()
            .join("\n");

        std::fs::write(path, packets)?;

        Ok(())
    }
}
