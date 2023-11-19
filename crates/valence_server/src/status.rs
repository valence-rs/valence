use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_protocol::packets::play::ClientStatusC2s;

use crate::event_loop::{EventLoopPreUpdate, PacketEvent};

pub struct StatusPlugin;

impl Plugin for StatusPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RequestRespawnEvent>()
            .add_event::<RequestStatsEvent>()
            .add_systems(EventLoopPreUpdate, handle_status);
    }
}

#[derive(Event, Copy, Clone, PartialEq, Eq, Debug)]
pub struct RequestRespawnEvent {
    pub client: Entity,
}

#[derive(Event, Copy, Clone, PartialEq, Eq, Debug)]
pub struct RequestStatsEvent {
    pub client: Entity,
}

fn handle_status(
    mut packets: EventReader<PacketEvent>,
    mut respawn_events: EventWriter<RequestRespawnEvent>,
    mut request_stats_events: EventWriter<RequestStatsEvent>,
) {
    for packet in packets.read() {
        if let Some(pkt) = packet.decode::<ClientStatusC2s>() {
            match pkt {
                ClientStatusC2s::PerformRespawn => respawn_events.send(RequestRespawnEvent {
                    client: packet.client,
                }),
                ClientStatusC2s::RequestStats => request_stats_events.send(RequestStatsEvent {
                    client: packet.client,
                }),
            }
        }
    }
}
