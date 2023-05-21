use valence_core::protocol::raw::RawBytes;
use valence_core::protocol::{packet_id, Decode, Encode};

use super::*;
use crate::event_loop::{EventLoopSchedule, EventLoopSet, PacketEvent};

pub(super) fn build(app: &mut App) {
    app.add_event::<CustomPayloadEvent>().add_system(
        handle_custom_payload
            .in_schedule(EventLoopSchedule)
            .in_base_set(EventLoopSet::PreUpdate),
    );
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CUSTOM_PAYLOAD_C2S)]
pub struct CustomPayloadC2s<'a> {
    pub channel: Ident<Cow<'a, str>>,
    pub data: RawBytes<'a>,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CUSTOM_PAYLOAD_S2C)]
pub struct CustomPayloadS2c<'a> {
    pub channel: Ident<Cow<'a, str>>,
    pub data: RawBytes<'a>,
}
