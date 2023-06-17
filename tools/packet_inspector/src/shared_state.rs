#![allow(clippy::mutable_key_type)]

use egui::Context;
use packet_inspector::Packet;
use std::{collections::HashMap, sync::RwLock};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct PacketFilter {
    inner: HashMap<Packet, bool>,
}

impl PacketFilter {
    pub fn new() -> Self {
        let mut inner = HashMap::new();

        for p in packet_inspector::STD_PACKETS.iter() {
            inner.insert(p.clone(), true);
        }

        Self { inner }
    }

    pub fn get(&self, packet: &Packet) -> Option<bool> {
        self.inner
            .iter()
            .find(|(k, _)| k.id == packet.id && k.side == packet.side && k.state == packet.state)
            .map(|(_, v)| v)
            .copied()
    }

    pub fn insert(&mut self, packet: Packet, value: bool) {
        self.inner.insert(packet, value);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Packet, &bool)> {
        self.inner.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Packet, &mut bool)> {
        self.inner.iter_mut()
    }
}

pub enum Event {
    StartListening,
    StopListening,
    PacketReceived,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SharedState {
    pub listener_addr: String,
    pub server_addr: String,
    pub autostart: bool,
    pub packet_filter: PacketFilter,
    #[serde(skip)]
    pub is_listening: bool,
    #[serde(skip)]
    pub selected_packet: Option<usize>,
    #[serde(skip)]
    pub packets: RwLock<Vec<Packet>>,
    #[serde(skip)]
    pub(super) receiver: Option<flume::Receiver<Event>>,
    #[serde(skip)]
    sender: Option<flume::Sender<Event>>,
    #[serde(skip)]
    pub ctx: Option<Context>,
}

impl Default for SharedState {
    fn default() -> Self {
        let (sender, receiver) = flume::unbounded();

        Self {
            listener_addr: "127.0.0.1:25566".to_string(),
            server_addr: "127.0.0.1:25565".to_string(),
            autostart: false,
            is_listening: false,
            packet_filter: PacketFilter::new(),
            selected_packet: None,
            packets: RwLock::new(Vec::new()),
            receiver: Some(receiver),
            sender: Some(sender),
            ctx: None,
        }
    }
}

#[allow(unused)]
impl SharedState {
    pub fn new(ctx: Context) -> Self {
        Self {
            ctx: Some(ctx),
            ..Self::default()
        }
    }
    pub(super) fn merge(mut self, other: Self) -> Self {
        self.ctx = other.ctx;
        self.sender = other.sender;
        self.receiver = other.receiver;

        // make a backup of self.packet_filter

        let mut packet_filter = PacketFilter::new();
        // iterate over packet_inspector::STD_PACKETS
        for p in packet_inspector::STD_PACKETS.iter() {
            // if the packet is in the current packet_filter
            if let Some(v) = self.packet_filter.get(p) {
                // insert it into packet_filter
                packet_filter.insert(p.clone(), v);
            } else {
                // otherwise insert it into packet_filter with a default value of true
                packet_filter.insert(p.clone(), true);
            }
        }

        self.packet_filter = packet_filter;

        self
    }

    pub fn send_event(&self, event: Event) {
        if let Some(sender) = &self.sender {
            sender.send(event);
        }
    }
}
