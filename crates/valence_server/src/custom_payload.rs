use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_protocol::packets::play::{CustomPayloadC2s, CustomPayloadS2c};
use valence_protocol::{Ident, WritePacket};

use crate::client::Client;
use crate::event_loop::{EventLoopPreUpdate, PacketEvent};

pub struct CustomPayloadPlugin;

impl Plugin for CustomPayloadPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CustomPayloadEvent>()
            .add_systems(EventLoopPreUpdate, handle_custom_payload);
    }
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
