use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::RwLock;

use owo_colors::{OwoColorize, Style};
use regex::Regex;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use valence::network::packet::{
    HandshakeC2s, LoginHelloC2s, LoginKeyC2s, LoginSuccessS2c, QueryPingC2s, QueryPongS2c,
    QueryRequestC2s, QueryResponseS2c,
};
use valence::protocol::decode::PacketDecoder;

use crate::packet_groups::{C2sPlayPacket, S2cLoginPacket, S2cPlayPacket};
use crate::packet_widget::{systemtime_strftime, PacketDirection};
use crate::MetaPacket;

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
    pub(crate) compression_threshold: Option<u32>,
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
        dec.set_compression(self.compression_threshold);
        dec.queue_slice(&self.packet_data);

        match dec.try_next_packet() {
            Ok(Some(data)) => data.into(),
            Ok(None) => vec![],
            Err(e) => {
                eprintln!("Error decoding packet: {e:#}");
                vec![]
            }
        }
    }

    pub fn get_packet_string(&self, formatted: bool) -> String {
        let mut dec = PacketDecoder::new();
        dec.set_compression(self.compression_threshold);
        dec.queue_slice(&self.packet_data);

        macro_rules! get {
            ($packet:ident) => {
                match dec.try_next_packet() {
                    Ok(Some(frame)) => {
                        if let Ok(pkt) =
                            <$packet as valence::protocol::Packet>::decode_packet(&mut &frame[..])
                        {
                            if formatted {
                                format!("{pkt:#?}")
                            } else {
                                format!("{pkt:?}")
                            }
                        } else {
                            stringify!($packet).into()
                        }
                    }
                    Ok(None) => stringify!($packet).into(),
                    Err(e) => format!("{e:#}"),
                }
            };
        }

        match self.stage {
            Stage::HandshakeC2s => get!(HandshakeC2s),
            Stage::QueryRequestC2s => get!(QueryRequestC2s),
            Stage::QueryResponseS2c => get!(QueryResponseS2c),
            Stage::QueryPingC2s => get!(QueryPingC2s),
            Stage::QueryPongS2c => get!(QueryPongS2c),
            Stage::LoginHelloC2s => get!(LoginHelloC2s),
            Stage::S2cLoginPacket => get!(S2cLoginPacket),
            Stage::LoginKeyC2s => get!(LoginKeyC2s),
            Stage::LoginSuccessS2c => get!(LoginSuccessS2c),
            Stage::C2sPlayPacket => get!(C2sPlayPacket),
            Stage::S2cPlayPacket => get!(S2cPlayPacket),
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
    pub visible_packets: RwLock<BTreeMap<MetaPacket, bool>>,
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
            visible_packets: RwLock::new(BTreeMap::new()),

            packet_count: RwLock::new(0),

            has_encryption_enabled_error: AtomicBool::new(false),

            c2s_style: Style::new().green(),
            s2c_style: Style::new().purple(),
        }
    }

    pub fn clear(&self) {
        self.last_packet.store(0, Ordering::Relaxed);
        *self.selected_packet.write().unwrap() = None;
        self.packets.write().unwrap().clear();
        if let ContextMode::Gui(ctx) = &self.mode {
            ctx.request_repaint();
        }
    }

    pub fn add(&self, mut packet: Packet) {
        match &self.mode {
            ContextMode::Gui(ctx) => {
                packet.id = self.last_packet.fetch_add(1, Ordering::Relaxed);
                self.packets.write().unwrap().push(packet);
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

    pub fn set_selected_packets(&self, packets: BTreeMap<MetaPacket, bool>) {
        *self.visible_packets.write().unwrap() = packets;
    }

    pub fn is_packet_hidden(&self, index: usize) -> bool {
        let packets = self.packets.read().unwrap();
        let packet = packets.get(index).expect("Packet not found");

        let visible_packets = self.visible_packets.read().unwrap();

        let meta_packet: MetaPacket = (*packet).clone().into();

        if let Some(visible) = visible_packets.get(&meta_packet) {
            if !visible {
                return true;
            }
        }

        let filter = self.filter.read().unwrap();
        let filter = filter.as_str();
        if !filter.is_empty()
            && packet
                .packet_name
                .to_lowercase()
                .contains(&filter.to_lowercase())
        {
            return true;
        }

        false
    }

    pub fn select_previous_packet(&self) {
        let mut selected_packet = self.selected_packet.write().unwrap();
        if let Some(idx) = *selected_packet {
            if idx > 0 {
                let mut new_index = idx - 1;
                while self.is_packet_hidden(new_index) {
                    if new_index == 0 {
                        new_index = idx;
                        break;
                    }
                    new_index -= 1;
                }
                *selected_packet = Some(new_index);
            }
        } else {
            let packets = self.packets.read().unwrap();
            if !packets.is_empty() {
                *selected_packet = Some(0);
            }
        }
    }

    pub fn select_next_packet(&self) {
        let mut selected_packet = self.selected_packet.write().unwrap();
        if let Some(idx) = *selected_packet {
            if idx < self.packets.read().unwrap().len() - 1 {
                let mut new_index = idx + 1;
                while self.is_packet_hidden(new_index) {
                    if new_index == self.packets.read().unwrap().len() - 1 {
                        new_index = idx;
                        break;
                    }
                    new_index += 1;
                }

                *selected_packet = Some(new_index);
            }
        } else {
            let packets = self.packets.read().unwrap();
            if !packets.is_empty() {
                *selected_packet = Some(1);
            }
        }
    }

    pub fn set_selected_packet(&self, idx: usize) {
        *self.selected_packet.write().unwrap() = Some(idx);
    }

    pub fn set_filter(&self, filter: String) {
        *self.filter.write().expect("Posisoned RwLock") = filter;
        *self.selected_packet.write().unwrap() = None;
    }

    pub fn save(&self, path: PathBuf) -> Result<(), std::io::Error> {
        let packets = self
            .packets
            .read()
            .unwrap()
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
