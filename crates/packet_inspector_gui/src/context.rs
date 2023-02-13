use std::sync::RwLock;

use crate::packet_widget::PacketDirection;

#[derive(Clone)]
pub struct Packet {
    pub(crate) id: usize,
    pub(crate) direction: PacketDirection,
    pub(crate) selected: bool,
    pub(crate) packet_type: u8,
    pub(crate) packet_name: String,
    pub(crate) packet: String,
}

impl Packet {
    pub(crate) fn selected(&mut self, value: bool) {
        self.selected = value;
    }
}

pub struct Context {
    pub selected_packet: RwLock<Option<usize>>,
    pub(crate) packets: RwLock<Vec<Packet>>,
    pub filter: RwLock<String>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            selected_packet: RwLock::new(None),
            packets: RwLock::new(Vec::new()),
            filter: RwLock::new("".into()),
        }
    }

    pub fn add(&self, packet: Packet) {
        self.packets.write().expect("Poisoned RwLock").push(packet);
    }

    pub fn set_selected_packet(&self, idx: usize) {
        *self.selected_packet.write().expect("Poisoned RwLock") = Some(idx);
    }

    pub fn set_filter(&self, filter: String) {
        *self.filter.write().expect("Posisoned RwLock") = filter;
        *self.selected_packet.write().expect("Poisoned RwLock") = None;
    }
}
