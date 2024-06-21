#![allow(clippy::mutable_key_type)]

use std::collections::HashMap;
use std::sync::RwLock;

use egui::Context;
use packet_inspector::Packet;

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct PacketFilter {
    inner: HashMap<Packet, bool>,
}

impl PacketFilter {
    pub(crate) fn new() -> Self {
        let mut inner = HashMap::new();

        for p in &packet_inspector::STD_PACKETS {
            inner.insert(p.clone(), true);
        }

        Self { inner }
    }

    pub(crate) fn get(&self, packet: &Packet) -> Option<bool> {
        self.inner
            .iter()
            .find(|(k, _)| k.id == packet.id && k.side == packet.side && k.state == packet.state)
            .map(|(_, v)| v)
            .copied()
    }

    pub(crate) fn insert(&mut self, packet: Packet, value: bool) {
        self.inner.insert(packet, value);
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&Packet, &bool)> {
        self.inner.iter()
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (&Packet, &mut bool)> {
        self.inner.iter_mut()
    }
}

pub(crate) enum Event {
    StartListening,
    StopListening,
    PacketReceived,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct SharedState {
    pub(crate) listener_addr: String,
    pub(crate) server_addr: String,
    pub(crate) autostart: bool,
    pub(crate) packet_filter: PacketFilter,
    pub(crate) packet_search: String,
    #[serde(skip)]
    pub(crate) is_listening: bool,
    #[serde(skip)]
    pub(crate) selected_packet: Option<usize>,
    #[serde(skip)]
    pub(crate) update_scroll: bool,
    #[serde(skip)]
    pub(crate) packets: RwLock<Vec<Packet>>,
    #[serde(skip)]
    pub(super) receiver: Option<flume::Receiver<Event>>,
    #[serde(skip)]
    sender: Option<flume::Sender<Event>>,
    #[serde(skip)]
    pub(crate) ctx: Option<Context>,
}

impl Default for SharedState {
    fn default() -> Self {
        let (sender, receiver) = flume::unbounded();

        Self {
            listener_addr: "127.0.0.1:25566".to_owned(),
            server_addr: "127.0.0.1:25565".to_owned(),
            autostart: false,
            is_listening: false,
            packet_search: String::new(),
            packet_filter: PacketFilter::new(),
            selected_packet: None,
            update_scroll: false,
            packets: RwLock::new(Vec::new()),
            receiver: Some(receiver),
            sender: Some(sender),
            ctx: None,
        }
    }
}

#[allow(unused)]
impl SharedState {
    pub(crate) fn new(ctx: Context) -> Self {
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
        for p in &packet_inspector::STD_PACKETS {
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

    pub(crate) fn send_event(&self, event: Event) {
        if let Some(sender) = &self.sender {
            sender.send(event);
        }
    }
}
