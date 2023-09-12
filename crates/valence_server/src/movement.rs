use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use tracing::warn;
use valence_entity::{HeadYaw, Look, OnGround, Position};
use valence_math::DVec3;
use valence_protocol::packets::play::player_position_look_s2c::PlayerPositionLookFlags;
use valence_protocol::packets::play::{
    FullC2s, LookAndOnGroundC2s, OnGroundOnlyC2s, PlayerPositionLookS2c, PlayerSpawnPositionS2c,
    PositionAndOnGroundC2s, TeleportConfirmC2s, VehicleMoveC2s,
};
use valence_protocol::{BlockPos, WritePacket};

use crate::event_loop::{EventLoopPreUpdate, PacketEvent};
use crate::Client;
use crate::layer::BroadcastLayerMessagesSet;

/// Handles client movement and teleports.
pub struct PositionPlugin;

/// When client positions are synchronized by sending the clientbound position
/// packet. This set also includes the system that updates the client's respawn
/// position.
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SyncPositionSet;

impl Plugin for PositionPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<MovementSettings>()
            .add_event::<MovementEvent>()
            .add_systems(EventLoopPreUpdate, (handle_teleport_confirmations, handle_client_movement))
            // Sync position after chunks are loaded so the client doesn't fall through the floor.
            // Setting the respawn position also closes the "downloading terrain" screen.
            .configure_set(PostUpdate, SyncPositionSet.after(BroadcastLayerMessagesSet))
            .add_systems(PostUpdate, (update_respawn_position, teleport).chain().in_set(SyncPositionSet));
    }
}

#[derive(Component, Debug)]
pub struct TeleportState {
    /// Counts up as teleports are made.
    teleport_id_counter: u32,
    /// The number of pending client teleports that have yet to receive a
    /// confirmation. Inbound client position packets should be ignored while
    /// this is nonzero.
    pending_teleports: u32,
    synced_pos: DVec3,
    synced_look: Look,
}

impl TeleportState {
    pub fn teleport_id_counter(&self) -> u32 {
        self.teleport_id_counter
    }

    pub fn pending_teleports(&self) -> u32 {
        self.pending_teleports
    }
}

impl Default for TeleportState {
    fn default() -> Self {
        Self {
            teleport_id_counter: 0,
            pending_teleports: 0,
            // Set initial synced pos and look to NaN so a teleport always happens when first
            // joining.
            synced_pos: DVec3::NAN,
            synced_look: Look {
                yaw: f32::NAN,
                pitch: f32::NAN,
            },
        }
    }
}

/// Syncs the client's position and look with the server.
///
/// This should happen after chunks are loaded so the client doesn't fall though
/// the floor.
#[allow(clippy::type_complexity)]
fn teleport(
    mut clients: Query<
        (&mut Client, &mut TeleportState, &Position, &Look),
        Or<(Changed<Position>, Changed<Look>)>,
    >,
) {
    for (mut client, mut state, pos, look) in &mut clients {
        let changed_pos = pos.0 != state.synced_pos;
        let changed_yaw = look.yaw != state.synced_look.yaw;
        let changed_pitch = look.pitch != state.synced_look.pitch;

        if changed_pos || changed_yaw || changed_pitch {
            state.synced_pos = pos.0;
            state.synced_look = *look;

            let flags = PlayerPositionLookFlags::new()
                .with_x(!changed_pos)
                .with_y(!changed_pos)
                .with_z(!changed_pos)
                .with_y_rot(!changed_yaw)
                .with_x_rot(!changed_pitch);

            client.write_packet(&PlayerPositionLookS2c {
                position: if changed_pos { pos.0 } else { DVec3::ZERO },
                yaw: if changed_yaw { look.yaw } else { 0.0 },
                pitch: if changed_pitch { look.pitch } else { 0.0 },
                flags,
                teleport_id: (state.teleport_id_counter as i32).into(),
            });

            state.pending_teleports = state.pending_teleports.wrapping_add(1);
            state.teleport_id_counter = state.teleport_id_counter.wrapping_add(1);
        }
    }
}

fn handle_teleport_confirmations(
    mut packets: EventReader<PacketEvent>,
    mut clients: Query<&mut TeleportState>,
    mut commands: Commands,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<TeleportConfirmC2s>() {
            if let Ok(mut state) = clients.get_mut(packet.client) {
                if state.pending_teleports == 0 {
                    warn!(
                        "unexpected teleport confirmation from client {:?}",
                        packet.client
                    );
                    commands.entity(packet.client).remove::<Client>();
                }

                let got = pkt.teleport_id.0 as u32;
                let expected = state
                    .teleport_id_counter
                    .wrapping_sub(state.pending_teleports);

                if got == expected {
                    state.pending_teleports -= 1;
                } else {
                    warn!(
                        "unexpected teleport ID for client {:?} (expected {expected}, got {got})",
                        packet.client
                    );
                    commands.entity(packet.client).remove::<Client>();
                }
            }
        }
    }
}

/// The position and angle that clients will respawn with. Also
/// controls the position that compasses point towards.
#[derive(Component, Copy, Clone, PartialEq, Default, Debug)]
pub struct RespawnPosition {
    /// The position that clients will respawn at. This can be changed at any
    /// time to set the position that compasses point towards.
    pub pos: BlockPos,
    /// The yaw angle that clients will respawn with (in degrees).
    pub yaw: f32,
}

/// Sets the client's respawn and compass position.
fn update_respawn_position(
    mut clients: Query<(&mut Client, &RespawnPosition), Changed<RespawnPosition>>,
) {
    for (mut client, respawn_pos) in &mut clients {
        client.write_packet(&PlayerSpawnPositionS2c {
            position: respawn_pos.pos,
            angle: respawn_pos.yaw,
        });
    }
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
