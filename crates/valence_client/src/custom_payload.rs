use valence_core::protocol::raw::RawBytes;
use valence_packet::packets::play::{CustomPayloadC2s, CustomPayloadS2c};

use super::*;
use crate::event_loop::{EventLoopPreUpdate, PacketEvent};

pub(super) fn build(app: &mut App) {
    app.add_event::<CustomPayloadEvent>()
        .add_systems(EventLoopPreUpdate, handle_custom_payload);
}

#[derive(Event, Clone, Debug)]
pub struct CustomPayloadEvent {
    pub client: Entity,
    pub channel: Ident<String>,
    pub data: Box<[u8]>,
}

impl Client {
    pub fn send_custom_payload(&mut self, channel: Ident<&str>, data: &[u8]) {
        self.write_packet(&CustomPayloadS2c {
            channel: channel.into(),
            data: data.into(),
        });
    }
}

fn handle_custom_payload(
    mut packets: EventReader<PacketEvent>,
    mut events: EventWriter<CustomPayloadEvent>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<CustomPayloadC2s>() {
            events.send(CustomPayloadEvent {
                client: packet.client,
                channel: pkt.channel.into(),
                data: pkt.data.0.into(),
            })
        }
    }
}
