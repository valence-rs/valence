use std::collections::VecDeque;
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
    pub(crate) use_compression: bool,
    pub(crate) packet_data: Vec<u8>,
    pub(crate) stage: Stage,
    pub(crate) created_at: OffsetDateTime,
}

#[derive(Clone)]
pub struct DisplayPacket {
    pub(crate) id: usize,
    pub(crate) direction: PacketDirection,
    pub(crate) selected: bool,
    pub(crate) packet_type: u8,
    pub(crate) packet_name: String,
    pub(crate) packet_str: String,
    pub(crate) created_at: OffsetDateTime,
}

impl From<Packet> for DisplayPacket {
    fn from(pkt: Packet) -> DisplayPacket {
        let packet = pkt.get_packet_string();

        // trim to some max length
        let packet = if packet.len() > 1024 {
            format!("{}\n...", &packet[..1024])
        } else {
            packet
        };

        let name = packet
            .split_once(|ch: char| !ch.is_ascii_alphanumeric())
            .map(|(fst, _)| fst)
            .unwrap_or(&packet);

        DisplayPacket {
            id: pkt.id,
            direction: pkt.direction,
            selected: false,
            packet_type: pkt.packet_data[0],
            packet_name: name.to_string(),
            packet_str: packet,
            created_at: pkt.created_at,
        }
    }
}

impl DisplayPacket {
    pub(crate) fn selected(&mut self, value: bool) {
        self.selected = value;
    }
}

impl Packet {
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
    pub(crate) process_packets: RwLock<VecDeque<Packet>>,
    pub(crate) packets: RwLock<Vec<DisplayPacket>>,
    pub(crate) packet_count: RwLock<usize>,
    pub filter: RwLock<String>,
    pub(crate) context: Option<egui::Context>,
}

impl Context {
    pub fn new(ctx: Option<egui::Context>) -> Self {
        Self {
            last_packet: AtomicUsize::new(0),
            selected_packet: RwLock::new(None),
            process_packets: RwLock::new(VecDeque::new()),
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
        self.process_packets
            .write()
            .expect("Poisoned RwLock")
            .push_back(packet);
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
            .map(|packet| packet.packet_str.replace("    ", "").replace('\n', " ")) // deformat the packet
            .collect::<Vec<String>>()
            .join("\n");

        std::fs::write(path, packets)?;

        Ok(())
    }
}
