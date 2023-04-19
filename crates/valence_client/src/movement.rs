use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use glam::DVec3;
use valence_core::packet::c2s::play::{
    Full, LookAndOnGround, OnGroundOnly, PositionAndOnGround, VehicleMoveC2s,
};
use valence_entity::{Look, Position, HeadYaw, OnGround};

use super::teleport::TeleportState;
use crate::event_loop::{EventLoopSchedule, EventLoopSet, PacketEvent};

pub(super) fn build(app: &mut App) {
    app.init_resource::<MovementSettings>()
        .add_event::<Movement>()
        .add_system(
            handle_client_movement
                .in_schedule(EventLoopSchedule)
                .in_base_set(EventLoopSet::PreUpdate),
        );
}

/// Configuration resource for client movement checks.
#[derive(Resource, Default)]
pub struct MovementSettings {
    // TODO
}

/// Event sent when a client successfully moves.
#[derive(Clone, Debug)]
pub struct Movement {
    pub client: Entity,
    pub position: DVec3,
    pub old_position: DVec3,
    pub look: Look,
    pub old_look: Look,
    pub on_ground: bool,
    pub old_on_ground: bool,
}

fn handle_client_movement(
    mut packets: EventReader<PacketEvent>,
    mut clients: Query<(
        &mut Position,
        &mut Look,
        &mut HeadYaw,
        &mut OnGround,
        &mut TeleportState,
    )>,
    mut movement_events: EventWriter<Movement>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<PositionAndOnGround>() {
            if let Ok((pos, look, head_yaw, on_ground, teleport_state)) =
                clients.get_mut(packet.client)
            {
                let mov = Movement {
                    client: packet.client,
                    position: pkt.position.into(),
                    old_position: pos.0,
                    look: *look,
                    old_look: *look,
                    on_ground: pkt.on_ground,
                    old_on_ground: on_ground.0,
                };

                handle(
                    mov,
                    pos,
                    look,
                    head_yaw,
                    on_ground,
                    teleport_state,
                    &mut movement_events,
                );
            }
        } else if let Some(pkt) = packet.decode::<Full>() {
            if let Ok((pos, look, head_yaw, on_ground, teleport_state)) =
                clients.get_mut(packet.client)
            {
                let mov = Movement {
                    client: packet.client,
                    position: pkt.position.into(),
                    old_position: pos.0,
                    look: Look {
                        yaw: pkt.yaw,
                        pitch: pkt.pitch,
                    },
                    old_look: *look,
                    on_ground: pkt.on_ground,
                    old_on_ground: on_ground.0,
                };

                handle(
                    mov,
                    pos,
                    look,
                    head_yaw,
                    on_ground,
                    teleport_state,
                    &mut movement_events,
                );
            }
        } else if let Some(pkt) = packet.decode::<LookAndOnGround>() {
            if let Ok((pos, look, head_yaw, on_ground, teleport_state)) =
                clients.get_mut(packet.client)
            {
                let mov = Movement {
                    client: packet.client,
                    position: pos.0,
                    old_position: pos.0,
                    look: Look {
                        yaw: pkt.yaw,
                        pitch: pkt.pitch,
                    },
                    old_look: *look,
                    on_ground: pkt.on_ground,
                    old_on_ground: on_ground.0,
                };

                handle(
                    mov,
                    pos,
                    look,
                    head_yaw,
                    on_ground,
                    teleport_state,
                    &mut movement_events,
                );
            }
        } else if let Some(pkt) = packet.decode::<OnGroundOnly>() {
            if let Ok((pos, look, head_yaw, on_ground, teleport_state)) =
                clients.get_mut(packet.client)
            {
                let mov = Movement {
                    client: packet.client,
                    position: pos.0,
                    old_position: pos.0,
                    look: *look,
                    old_look: *look,
                    on_ground: pkt.on_ground,
                    old_on_ground: on_ground.0,
                };

                handle(
                    mov,
                    pos,
                    look,
                    head_yaw,
                    on_ground,
                    teleport_state,
                    &mut movement_events,
                );
            }
        } else if let Some(pkt) = packet.decode::<VehicleMoveC2s>() {
            if let Ok((pos, look, head_yaw, on_ground, teleport_state)) =
                clients.get_mut(packet.client)
            {
                let mov = Movement {
                    client: packet.client,
                    position: pkt.position.into(),
                    old_position: pos.0,
                    look: Look {
                        yaw: pkt.yaw,
                        pitch: pkt.pitch,
                    },
                    old_look: *look,
                    on_ground: on_ground.0,
                    old_on_ground: on_ground.0,
                };

                handle(
                    mov,
                    pos,
                    look,
                    head_yaw,
                    on_ground,
                    teleport_state,
                    &mut movement_events,
                );
            }
        }
    }
}

fn handle(
    mov: Movement,
    mut pos: Mut<Position>,
    mut look: Mut<Look>,
    mut head_yaw: Mut<HeadYaw>,
    mut on_ground: Mut<OnGround>,
    mut teleport_state: Mut<TeleportState>,
    movement_events: &mut EventWriter<Movement>,
) {
    if teleport_state.pending_teleports() != 0 {
        return;
    }

    // TODO: check that the client isn't moving too fast / flying.
    // TODO: check that the client isn't clipping through blocks.

    pos.set_if_neq(Position(mov.position));
    teleport_state.synced_pos = mov.position;
    look.set_if_neq(mov.look);
    teleport_state.synced_look = mov.look;
    head_yaw.set_if_neq(HeadYaw(mov.look.yaw));
    on_ground.set_if_neq(OnGround(mov.on_ground));

    movement_events.send(mov);
}
