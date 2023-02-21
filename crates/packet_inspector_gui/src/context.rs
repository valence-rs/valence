use std::{
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
};

use time::OffsetDateTime;
use valence_protocol::{
    packets::{
        c2s::{
            handshake::Handshake,
            login::{EncryptionResponse, LoginStart},
            status::{PingRequest, StatusRequest},
        },
        s2c::{
            login::LoginSuccess,
            status::{PingResponse, StatusResponse},
        },
        C2sPlayPacket, S2cLoginPacket, S2cPlayPacket,
    },
    PacketDecoder,
};

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
    pub(crate) packet_type: u8,
    pub(crate) packet_name: Arc<Mutex<Option<String>>>,
    pub(crate) packet_str: Arc<Mutex<Option<String>>>,
    pub(crate) packet_data: Vec<u8>,
    pub(crate) stage: Stage,
    pub(crate) created_at: OffsetDateTime,
}

impl Packet {
    pub(crate) fn selected(&mut self, value: bool) {
        self.selected = value;
    }

    pub fn get_name(&self) -> String {
        let mut name = self.packet_name.lock().expect("Poisoned Mutex");
        if name.is_none() {
            let packet = self.as_formatted_string_internal();

            let packet_name = packet
                .split_once(|ch: char| !ch.is_ascii_alphanumeric())
                .map(|(fst, _)| fst)
                .unwrap_or(&packet);

            *name = Some(packet_name.to_string());
        }

        name.clone().unwrap()
    }

    pub fn get_packet_string(&self) -> String {
        let mut packet_str = self.packet_str.lock().expect("Poisoned Mutex");
        if packet_str.is_none() {
            let packet = self.as_formatted_string_internal();
            *packet_str = Some(packet);
        }

        packet_str.clone().unwrap()
    }

    pub fn get_packet_string_deformatted(&self) -> String {
        let mut packet_str = self.packet_str.lock().expect("Poisoned Mutex");
        if packet_str.is_none() {
            let packet = self.as_formatted_string_internal();
            *packet_str = Some(packet);
        }

        let packet_str = packet_str.clone().unwrap();

        // probably not the cleanest way to do this, but it avoids needing to decode the packet again
        let packet_str = packet_str.replace("    ", "").replace("\n", " ");

        packet_str
    }

    fn as_formatted_string_internal(&self) -> String {
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
    pub selected_packet: RwLock<Option<usize>>,
    pub(crate) packets: RwLock<Vec<Packet>>,
    pub(crate) packet_count: RwLock<usize>,
    pub filter: RwLock<String>,
    context: Option<egui::Context>,
}

impl Context {
    pub fn new(ctx: Option<egui::Context>) -> Self {
        Self {
            selected_packet: RwLock::new(None),
            packets: RwLock::new(Vec::new()),
            filter: RwLock::new("".into()),
            context: ctx,
            packet_count: RwLock::new(0),
        }
    }

    pub fn clear(&self) {
        *self.selected_packet.write().expect("Poisoned RwLock") = None;
        self.packets.write().expect("Poisoned RwLock").clear();
        if let Some(ctx) = &self.context {
            ctx.request_repaint();
        }
    }

    pub fn add(&self, mut packet: Packet) {
        packet.id = self.packets.read().expect("Poisened RwLock").len();
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
            .filter(|packet| packet.get_name() != "ChunkDataAndUpdateLight") // temporarily blacklisting this packet because HUGE
            .map(|packet| packet.get_packet_string_deformatted())
            .collect::<Vec<String>>()
            .join("\n");

        std::fs::write(path, packets)?;

        Ok(())
    }
}
