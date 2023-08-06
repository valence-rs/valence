use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_entity::{HeadYaw, Look, OnGround, Position};
use valence_math::DVec3;
use valence_packet::packets::play::{
    FullC2s, LookAndOnGroundC2s, OnGroundOnlyC2s, PositionAndOnGroundC2s, VehicleMoveC2s,
};

use super::teleport::TeleportState;
use crate::event_loop::{EventLoopPreUpdate, PacketEvent};

pub(super) fn build(app: &mut App) {
    app.init_resource::<MovementSettings>()
        .add_event::<MovementEvent>()
        .add_systems(EventLoopPreUpdate, handle_client_movement);
}

/// Configuration resource for client movement checks.
#[derive(Resource, Default)]
pub struct MovementSettings {
    // TODO
}

/// Event sent when a client successfully moves.
#[derive(Event, Clone, Debug)]
pub struct MovementEvent {
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
    mut movement_events: EventWriter<MovementEvent>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<PositionAndOnGroundC2s>() {
            if let Ok((pos, look, head_yaw, on_ground, teleport_state)) =
                clients.get_mut(packet.client)
            {
                let mov = MovementEvent {
                    client: packet.client,
                    position: pkt.position,
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
        } else if let Some(pkt) = packet.decode::<FullC2s>() {
            if let Ok((pos, look, head_yaw, on_ground, teleport_state)) =
                clients.get_mut(packet.client)
            {
                let mov = MovementEvent {
                    client: packet.client,
                    position: pkt.position,
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
        } else if let Some(pkt) = packet.decode::<LookAndOnGroundC2s>() {
            if let Ok((pos, look, head_yaw, on_ground, teleport_state)) =
                clients.get_mut(packet.client)
            {
                let mov = MovementEvent {
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
        } else if let Some(pkt) = packet.decode::<OnGroundOnlyC2s>() {
            if let Ok((pos, look, head_yaw, on_ground, teleport_state)) =
                clients.get_mut(packet.client)
            {
                let mov = MovementEvent {
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
                let mov = MovementEvent {
                    client: packet.client,
                    position: pkt.position,
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
    mov: MovementEvent,
    mut pos: Mut<Position>,
    mut look: Mut<Look>,
    mut head_yaw: Mut<HeadYaw>,
    mut on_ground: Mut<OnGround>,
    mut teleport_state: Mut<TeleportState>,
    movement_events: &mut EventWriter<MovementEvent>,
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
