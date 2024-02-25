use std::time::{Duration, Instant};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use derive_more::Deref;
use tracing::warn;
use valence_protocol::packets::play::{KeepAliveC2s, KeepAliveS2c};
use valence_protocol::WritePacket;

use crate::client::{Client, UpdateClientsSet};
use crate::event_loop::{EventLoopPreUpdate, PacketEvent};

pub struct KeepalivePlugin;

impl Plugin for KeepalivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KeepaliveSettings>()
            .add_systems(PostUpdate, send_keepalive.in_set(UpdateClientsSet))
            .add_systems(EventLoopPreUpdate, handle_keepalive_response);
    }
}

#[derive(Resource, Debug)]
pub struct KeepaliveSettings {
    // How long to wait before sending keepalives and how long to wait for a response.
    pub period: Duration,
}

impl Default for KeepaliveSettings {
    fn default() -> Self {
        Self {
            period: Duration::from_secs(8),
        }
    }
}

#[derive(Component, Debug)]
pub struct KeepaliveState {
    got_keepalive: bool,
    last_keepalive_id: u64,
    last_send: Instant,
}

/// Delay measured in milliseconds. Negative values indicate absence.
#[derive(Component, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deref)]
pub struct Ping(pub i32);

impl Default for Ping {
    fn default() -> Self {
        Self(-1)
    }
}

impl KeepaliveState {
    pub(super) fn new() -> Self {
        Self {
            got_keepalive: true,
            last_keepalive_id: 0,
            last_send: Instant::now(),
        }
    }

    /// When the last keepalive was sent for this client.
    pub fn last_send(&self) -> Instant {
        self.last_send
    }
}

fn send_keepalive(
    mut clients: Query<(Entity, &mut Client, &mut KeepaliveState)>,
    settings: Res<KeepaliveSettings>,
    mut commands: Commands,
) {
    let now = Instant::now();

    for (entity, mut client, mut state) in &mut clients {
        if now.duration_since(state.last_send) >= settings.period {
            if state.got_keepalive {
                let id = rand::random();
                client.write_packet(&KeepAliveS2c { id });

                state.got_keepalive = false;
                state.last_keepalive_id = id;
                state.last_send = now;
            } else {
                let millis = settings.period.as_millis();
                warn!("Client {entity:?} timed out: no keepalive response after {millis}ms");
                commands.entity(entity).remove::<Client>();
            }
        }
    }
}

fn handle_keepalive_response(
    mut packets: EventReader<PacketEvent>,
    mut clients: Query<(Entity, &mut KeepaliveState, &mut Ping)>,
    mut commands: Commands,
) {
    for packet in packets.read() {
        let Some(pkt) = packet.decode::<KeepAliveC2s>() else {
            continue;
        };

        let Ok((entity, mut state, mut ping)) = clients.get_mut(packet.client) else {
            continue;
        };

        if state.got_keepalive {
            warn!("unexpected keepalive from client {entity:?}");
            commands.entity(entity).remove::<Client>();
        } else if pkt.id != state.last_keepalive_id {
            warn!(
                "keepalive IDs don't match for client {entity:?} (expected {}, got {})",
                state.last_keepalive_id, pkt.id,
            );
            commands.entity(entity).remove::<Client>();
        } else {
            state.got_keepalive = true;
            ping.0 = state.last_send.elapsed().as_millis() as i32;
        }
    }
}
