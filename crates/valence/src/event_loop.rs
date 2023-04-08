use std::time::Instant;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;
use bevy_ecs::system::SystemState;
use bytes::Bytes;
use tracing::{debug, warn};
use valence_protocol::{Decode, Packet};

use crate::client::Client;

pub(crate) struct EventLoopPlugin;

impl Plugin for EventLoopPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.configure_set(RunEventLoopSet.in_base_set(CoreSet::PreUpdate))
            .add_system(run_event_loop.in_set(RunEventLoopSet))
            .add_event::<PacketEvent>();

        // Add the event loop schedule.
        let mut event_loop = Schedule::new();
        event_loop.set_default_base_set(EventLoopSet::Update);
        event_loop.configure_sets((
            EventLoopSet::PreUpdate.before(EventLoopSet::Update),
            EventLoopSet::Update.before(EventLoopSet::PostUpdate),
            EventLoopSet::PostUpdate,
        ));

        app.add_schedule(EventLoopSchedule, event_loop);
    }
}

/// The [`ScheduleLabel`] for the event loop [`Schedule`].
#[derive(ScheduleLabel, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct EventLoopSchedule;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
#[system_set(base)]
pub enum EventLoopSet {
    PreUpdate,
    #[default]
    Update,
    PostUpdate,
}

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct RunEventLoopSet;

#[derive(Clone, Debug)]
pub struct PacketEvent {
    /// The client this packet originated from.
    pub client: Entity,
    /// The moment in time this packet arrived.
    pub timestamp: Instant,
    /// This packet's ID.
    pub id: i32,
    /// The content of the packet, excluding the leading varint packet ID.
    pub data: Bytes,
}

impl PacketEvent {
    /// Attempts to decode this packet as the packet `P`.
    ///
    /// If the packet ID is mismatched or an error occurs, `None` is returned.
    /// Otherwise, `Some` is returned containing the decoded packet.
    #[inline]
    pub fn decode<'a, P>(&'a self) -> Option<P>
    where
        P: Packet<'a> + Decode<'a>,
    {
        if self.id == P::PACKET_ID {
            let mut r = &self.data[..];

            match P::decode(&mut r) {
                Ok(pkt) => {
                    if r.is_empty() {
                        return Some(pkt);
                    }

                    warn!(
                        "missed {} bytes while decoding packet {} (ID = {})",
                        r.len(),
                        pkt.packet_name(),
                        P::PACKET_ID
                    );
                    debug!("complete packet after partial decode: {pkt:?}");
                }
                Err(e) => {
                    warn!("failed to decode packet with ID of {}: {e:#}", P::PACKET_ID);
                }
            }
        }

        None
    }
}

/// An exclusive system for running the event loop schedule.
pub(crate) fn run_event_loop(
    world: &mut World,
    state: &mut SystemState<(
        Query<(Entity, &mut Client)>,
        EventWriter<PacketEvent>,
        Commands,
    )>,
    mut check_again: Local<Vec<(Entity, usize)>>,
) {
    debug_assert!(check_again.is_empty());

    let (mut clients, mut event_writer, mut commands) = state.get_mut(world);

    for (entity, mut client) in &mut clients {
        match client.connection_mut().try_recv() {
            Ok(Some(pkt)) => {
                event_writer.send(PacketEvent {
                    client: entity,
                    timestamp: pkt.timestamp,
                    id: pkt.id,
                    data: pkt.data,
                });

                let remaining = client.connection().len();

                if remaining > 0 {
                    check_again.push((entity, remaining));
                }
            }
            Ok(None) => {}
            Err(e) => {
                // Client is disconnected.
                debug!("disconnecting client: {e:#}");
                commands.entity(entity).remove::<Client>();
            }
        }
    }

    state.apply(world);
    world.run_schedule(EventLoopSchedule);

    while !check_again.is_empty() {
        let (mut clients, mut event_writer, mut commands) = state.get_mut(world);

        check_again.retain_mut(|(entity, remaining)| {
            debug_assert!(*remaining > 0);

            if let Ok((_, mut client)) = clients.get_mut(*entity) {
                match client.connection_mut().try_recv() {
                    Ok(Some(pkt)) => {
                        event_writer.send(PacketEvent {
                            client: *entity,
                            timestamp: pkt.timestamp,
                            id: pkt.id,
                            data: pkt.data,
                        });
                        *remaining -= 1;
                        // Keep looping as long as there are packets to process this tick.
                        *remaining > 0
                    }
                    Ok(None) => false,
                    Err(e) => {
                        // Client is disconnected.
                        debug!("disconnecting client: {e:#}");
                        commands.entity(*entity).remove::<Client>();
                        false
                    }
                }
            } else {
                // Client must have been deleted in the last run of the schedule.
                false
            }
        });

        state.apply(world);
        world.run_schedule(EventLoopSchedule);
    }
}
