use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_entity::entity::Flags;
use valence_entity::{entity, Pose};
pub use valence_protocol::packets::play::client_command_c2s::PlayerCommand;
use valence_protocol::packets::play::PlayerCommandC2s;

use crate::event_loop::{EventLoopPreUpdate, PacketEvent};

pub struct ClientCommandPlugin;

impl Plugin for ClientCommandPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SprintEvent>()
            .add_event::<SneakEvent>()
            .add_event::<JumpWithHorseEvent>()
            .add_event::<LeaveBedEvent>()
            .add_systems(EventLoopPreUpdate, handle_client_command);
    }
}

#[derive(Event, Copy, Clone, PartialEq, Eq, Debug)]
pub struct SprintEvent {
    pub client: Entity,
    pub state: SprintState,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SprintState {
    Start,
    Stop,
}

#[derive(Event, Copy, Clone, PartialEq, Eq, Debug)]
pub struct SneakEvent {
    pub client: Entity,
    pub state: SneakState,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SneakState {
    Start,
    Stop,
}

#[derive(Event, Copy, Clone, PartialEq, Eq, Debug)]
pub struct JumpWithHorseEvent {
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

#[derive(Event, Copy, Clone, PartialEq, Eq, Debug)]
pub struct LeaveBedEvent {
    pub client: Entity,
}

fn handle_client_command(
    mut packets: EventReader<PacketEvent>,
    mut clients: Query<(&mut entity::Pose, &mut Flags)>,
    mut sprinting_events: EventWriter<SprintEvent>,
    mut sneaking_events: EventWriter<SneakEvent>,
    mut jump_with_horse_events: EventWriter<JumpWithHorseEvent>,
    mut leave_bed_events: EventWriter<LeaveBedEvent>,
) {
    for packet in packets.read() {
        if let Some(pkt) = packet.decode::<PlayerCommandC2s>() {
            match pkt.action {
                PlayerCommand::StartSneaking => {
                    if let Ok((mut pose, mut flags)) = clients.get_mut(packet.client) {
                        pose.0 = Pose::Sneaking;
                        flags.set_sneaking(true);
                    }

                    sneaking_events.send(SneakEvent {
                        client: packet.client,
                        state: SneakState::Start,
                    });
                }
                PlayerCommand::StopSneaking => {
                    if let Ok((mut pose, mut flags)) = clients.get_mut(packet.client) {
                        pose.0 = Pose::Standing;
                        flags.set_sneaking(false);
                    }

                    sneaking_events.send(SneakEvent {
                        client: packet.client,
                        state: SneakState::Stop,
                    });
                }
                PlayerCommand::LeaveBed => {
                    leave_bed_events.send(LeaveBedEvent {
                        client: packet.client,
                    });
                }
                PlayerCommand::StartSprinting => {
                    if let Ok((_, mut flags)) = clients.get_mut(packet.client) {
                        flags.set_sprinting(true);
                    }

                    sprinting_events.send(SprintEvent {
                        client: packet.client,
                        state: SprintState::Start,
                    });
                }
                PlayerCommand::StopSprinting => {
                    if let Ok((_, mut flags)) = clients.get_mut(packet.client) {
                        flags.set_sprinting(false);
                    }

                    sprinting_events.send(SprintEvent {
                        client: packet.client,
                        state: SprintState::Stop,
                    });
                }
                PlayerCommand::StartJumpWithHorse => {
                    jump_with_horse_events.send(JumpWithHorseEvent {
                        client: packet.client,
                        state: JumpWithHorseState::Start {
                            power: pkt.jump_boost.0 as u8,
                        },
                    });
                }
                PlayerCommand::StopJumpWithHorse => {
                    jump_with_horse_events.send(JumpWithHorseEvent {
                        client: packet.client,
                        state: JumpWithHorseState::Stop,
                    });
                }
                PlayerCommand::OpenHorseInventory => {} // TODO
                PlayerCommand::StartFlyingWithElytra => {
                    if let Ok((mut pose, _)) = clients.get_mut(packet.client) {
                        pose.0 = Pose::FallFlying;
                    }

                    // TODO.
                }
            }
        }
    }
}
