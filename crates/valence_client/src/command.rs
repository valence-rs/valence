use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_core::packet::c2s::play::client_command::Action;
use valence_core::packet::c2s::play::ClientCommandC2s;

use valence_entity::entity::Flags;
use valence_entity::{entity, Pose};
use crate::event_loop::{EventLoopSchedule, EventLoopSet, PacketEvent};

pub(super) fn build(app: &mut App) {
    app.add_event::<Sprinting>()
        .add_event::<Sneaking>()
        .add_event::<JumpWithHorse>()
        .add_event::<LeaveBed>()
        .add_system(
            handle_client_command
                .in_schedule(EventLoopSchedule)
                .in_base_set(EventLoopSet::PreUpdate),
        );
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Sprinting {
    pub client: Entity,
    pub state: SprintState,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SprintState {
    Start,
    Stop,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Sneaking {
    pub client: Entity,
    pub state: SneakState,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SneakState {
    Start,
    Stop,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct JumpWithHorse {
    pub client: Entity,
    pub state: JumpWithHorseState,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum JumpWithHorseState {
    Start {
        /// The power of the horse jump in `0..=100`.
        power: u8,
    },
    Stop,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct LeaveBed {
    pub client: Entity,
}

fn handle_client_command(
    mut packets: EventReader<PacketEvent>,
    mut clients: Query<(&mut entity::Pose, &mut Flags)>,
    mut sprinting_events: EventWriter<Sprinting>,
    mut sneaking_events: EventWriter<Sneaking>,
    mut jump_with_horse_events: EventWriter<JumpWithHorse>,
    mut leave_bed_events: EventWriter<LeaveBed>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<ClientCommandC2s>() {
            match pkt.action {
                Action::StartSneaking => {
                    if let Ok((mut pose, mut flags)) = clients.get_mut(packet.client) {
                        pose.0 = Pose::Sneaking;
                        flags.set_sneaking(true);
                    }

                    sneaking_events.send(Sneaking {
                        client: packet.client,
                        state: SneakState::Start,
                    })
                }
                Action::StopSneaking => {
                    if let Ok((mut pose, mut flags)) = clients.get_mut(packet.client) {
                        pose.0 = Pose::Standing;
                        flags.set_sneaking(false);
                    }

                    sneaking_events.send(Sneaking {
                        client: packet.client,
                        state: SneakState::Stop,
                    })
                }
                Action::LeaveBed => leave_bed_events.send(LeaveBed {
                    client: packet.client,
                }),
                Action::StartSprinting => {
                    if let Ok((_, mut flags)) = clients.get_mut(packet.client) {
                        flags.set_sprinting(true);
                    }

                    sprinting_events.send(Sprinting {
                        client: packet.client,
                        state: SprintState::Start,
                    });
                }
                Action::StopSprinting => {
                    if let Ok((_, mut flags)) = clients.get_mut(packet.client) {
                        flags.set_sprinting(false);
                    }

                    sprinting_events.send(Sprinting {
                        client: packet.client,
                        state: SprintState::Stop,
                    })
                }
                Action::StartJumpWithHorse => jump_with_horse_events.send(JumpWithHorse {
                    client: packet.client,
                    state: JumpWithHorseState::Start {
                        power: pkt.jump_boost.0 as u8,
                    },
                }),
                Action::StopJumpWithHorse => jump_with_horse_events.send(JumpWithHorse {
                    client: packet.client,
                    state: JumpWithHorseState::Stop,
                }),
                Action::OpenHorseInventory => {} // TODO
                Action::StartFlyingWithElytra => {
                    if let Ok((mut pose, _)) = clients.get_mut(packet.client) {
                        pose.0 = Pose::FallFlying;
                    }

                    // TODO.
                }
            }
        }
    }
}
