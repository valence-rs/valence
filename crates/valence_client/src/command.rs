use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{packet_id, Decode, Encode, Packet};
use valence_entity::entity::Flags;
use valence_entity::{entity, Pose};

use crate::event_loop::{EventLoopSchedule, EventLoopSet, PacketEvent};

pub(super) fn build(app: &mut App) {
    app.add_event::<SprintEvent>()
        .add_event::<SneakEvent>()
        .add_event::<JumpWithHorseEvent>()
        .add_event::<LeaveBedEvent>()
        .add_system(
            handle_client_command
                .in_schedule(EventLoopSchedule)
                .in_base_set(EventLoopSet::PreUpdate),
        );
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct SprintEvent {
    pub client: Entity,
    pub state: SprintState,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SprintState {
    Start,
    Stop,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct SneakEvent {
    pub client: Entity,
    pub state: SneakState,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SneakState {
    Start,
    Stop,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<ClientCommandC2s>() {
            match pkt.action {
                ClientCommand::StartSneaking => {
                    if let Ok((mut pose, mut flags)) = clients.get_mut(packet.client) {
                        pose.0 = Pose::Sneaking;
                        flags.set_sneaking(true);
                    }

                    sneaking_events.send(SneakEvent {
                        client: packet.client,
                        state: SneakState::Start,
                    })
                }
                ClientCommand::StopSneaking => {
                    if let Ok((mut pose, mut flags)) = clients.get_mut(packet.client) {
                        pose.0 = Pose::Standing;
                        flags.set_sneaking(false);
                    }

                    sneaking_events.send(SneakEvent {
                        client: packet.client,
                        state: SneakState::Stop,
                    })
                }
                ClientCommand::LeaveBed => leave_bed_events.send(LeaveBedEvent {
                    client: packet.client,
                }),
                ClientCommand::StartSprinting => {
                    if let Ok((_, mut flags)) = clients.get_mut(packet.client) {
                        flags.set_sprinting(true);
                    }

                    sprinting_events.send(SprintEvent {
                        client: packet.client,
                        state: SprintState::Start,
                    });
                }
                ClientCommand::StopSprinting => {
                    if let Ok((_, mut flags)) = clients.get_mut(packet.client) {
                        flags.set_sprinting(false);
                    }

                    sprinting_events.send(SprintEvent {
                        client: packet.client,
                        state: SprintState::Stop,
                    })
                }
                ClientCommand::StartJumpWithHorse => {
                    jump_with_horse_events.send(JumpWithHorseEvent {
                        client: packet.client,
                        state: JumpWithHorseState::Start {
                            power: pkt.jump_boost.0 as u8,
                        },
                    })
                }
                ClientCommand::StopJumpWithHorse => {
                    jump_with_horse_events.send(JumpWithHorseEvent {
                        client: packet.client,
                        state: JumpWithHorseState::Stop,
                    })
                }
                ClientCommand::OpenHorseInventory => {} // TODO
                ClientCommand::StartFlyingWithElytra => {
                    if let Ok((mut pose, _)) = clients.get_mut(packet.client) {
                        pose.0 = Pose::FallFlying;
                    }

                    // TODO.
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CLIENT_COMMAND_C2S)]
pub struct ClientCommandC2s {
    pub entity_id: VarInt,
    pub action: ClientCommand,
    pub jump_boost: VarInt,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum ClientCommand {
    StartSneaking,
    StopSneaking,
    LeaveBed,
    StartSprinting,
    StopSprinting,
    StartJumpWithHorse,
    StopJumpWithHorse,
    OpenHorseInventory,
    StartFlyingWithElytra,
}
