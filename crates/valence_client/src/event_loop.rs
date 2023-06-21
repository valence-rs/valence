use std::time::Instant;

use bevy_app::prelude::*;
use bevy_app::MainScheduleOrder;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;
use bevy_ecs::system::SystemState;
use bytes::Bytes;
use tracing::{debug, warn};
use valence_core::protocol::{Decode, Packet};

use crate::Client;

pub(super) fn build(app: &mut App) {
    app.add_event::<PacketEvent>()
        .add_schedule(RunEventLoop, Schedule::new())
        .add_schedule(EventLoopPreUpdate, Schedule::new())
        .add_schedule(EventLoopUpdate, Schedule::new())
        .add_schedule(EventLoopPostUpdate, Schedule::new())
        .add_systems(RunEventLoop, run_event_loop);

    app.world
        .resource_mut::<MainScheduleOrder>()
        .insert_after(PreUpdate, RunEventLoop);
}

/// The schedule responsible for running [`EventLoopPreUpdate`],
/// [`EventLoopUpdate`], and [`EventLoopPostUpdate`].
///
/// This schedule is situated between [`PreUpdate`] and [`Update`].
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RunEventLoop;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EventLoopPreUpdate;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EventLoopUpdate;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EventLoopPostUpdate;

#[derive(Event, Clone, Debug)]
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
        P: Packet + Decode<'a>,
    {
        if self.id == P::ID {
            let mut r = &self.data[..];

            match P::decode(&mut r) {
                Ok(pkt) => {
                    if r.is_empty() {
                        return Some(pkt);
                    }

                    warn!(
                        "missed {} bytes while decoding packet {} (ID = {})",
                        r.len(),
                        P::NAME,
                        P::ID
                    );
                    debug!("complete packet after partial decode: {pkt:?}");
                }
                Err(e) => {
                    warn!("failed to decode packet with ID of {}: {e:#}", P::ID);
                }
            }
        }

        None
    }
}

fn run_event_loop_schedules(world: &mut World) {
    world.run_schedule(EventLoopPreUpdate);
    world.run_schedule(EventLoopUpdate);
    world.run_schedule(EventLoopPostUpdate);
}

/// An exclusive system for running the event loop schedule.
#[allow(clippy::type_complexity)]
fn run_event_loop(
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
                    data: pkt.body,
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
    run_event_loop_schedules(world);

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
                            data: pkt.body,
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
        run_event_loop_schedules(world);
    }
}
