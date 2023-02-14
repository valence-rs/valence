use std::{path::PathBuf, sync::RwLock};

use time::OffsetDateTime;

use crate::packet_widget::PacketDirection;

#[derive(Clone)]
pub struct Packet {
    pub(crate) id: usize,
    pub(crate) direction: PacketDirection,
    pub(crate) selected: bool,
    pub(crate) packet_type: u8,
    pub(crate) packet_name: String,
    pub(crate) packet: String,
    pub(crate) packet_raw: String,
    pub(crate) created_at: OffsetDateTime,
}

impl Packet {
    pub(crate) fn selected(&mut self, value: bool) {
        self.selected = value;
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
            .filter(|packet| packet.packet_name != "ChunkDataAndUpdateLight") // temporarily blacklisting this packet because HUGE
            .map(|packet| packet.packet_raw.clone())
            .collect::<Vec<String>>()
            .join("\n");

        std::fs::write(path, packets)?;

        Ok(())
    }
}
