use valence_protocol::packet::c2s::play::KeepAliveC2s;

use super::*;
use crate::event_loop::{EventLoopSchedule, EventLoopSet, PacketEvent};

pub(super) fn build(app: &mut App) {
    app.add_system(
        send_keepalive
            .in_base_set(CoreSet::PostUpdate)
            .before(FlushPacketsSet),
    )
    .add_system(
        handle_keepalive_response
            .in_base_set(EventLoopSet::PreUpdate)
            .in_schedule(EventLoopSchedule),
    );
}

#[derive(Component, Debug)]
pub struct KeepaliveState {
    got_keepalive: bool,
    last_keepalive_id: u64,
    keepalive_sent_time: Instant,
}

impl KeepaliveState {
    pub(super) fn new() -> Self {
        Self {
            got_keepalive: true,
            last_keepalive_id: 0,
            keepalive_sent_time: Instant::now(),
        }
    }
}

fn send_keepalive(
    mut clients: Query<(Entity, &mut Client, &mut KeepaliveState)>,
    server: Res<Server>,
    mut commands: Commands,
) {
    if server.current_tick() % (server.tps() * 10) == 0 {
        let mut rng = rand::thread_rng();
        let now = Instant::now();

        for (entity, mut client, mut state) in &mut clients {
            if state.got_keepalive {
                let id = rng.gen();
                client.write_packet(&KeepAliveS2c { id });

                state.got_keepalive = false;
                state.last_keepalive_id = id;
                state.keepalive_sent_time = now;
            } else {
                warn!("Client {entity:?} timed out (no keepalive response)");
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
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<KeepAliveC2s>() {
            if let Ok((client, mut state, mut ping)) = clients.get_mut(packet.client) {
                if state.got_keepalive {
                    warn!("unexpected keepalive from client {client:?}");
                    commands.entity(client).remove::<Client>();
                } else if pkt.id != state.last_keepalive_id {
                    warn!(
                        "keepalive IDs don't match for client {client:?} (expected {}, got {})",
                        state.last_keepalive_id, pkt.id,
                    );
                    commands.entity(client).remove::<Client>();
                } else {
                    state.got_keepalive = true;
                    ping.0 = state.keepalive_sent_time.elapsed().as_millis() as i32;
                }
            }
        }
    }
}
