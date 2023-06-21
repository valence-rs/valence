use bitfield_struct::bitfield;
use glam::DVec3;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{packet_id, Decode, Encode, Packet};

use super::*;
use crate::event_loop::{EventLoopPreUpdate, PacketEvent};

pub(super) fn build(app: &mut App) {
    app.add_systems(
        PostUpdate,
        teleport.after(update_view).in_set(UpdateClientsSet),
    )
    .add_systems(EventLoopPreUpdate, handle_teleport_confirmations);
}

#[derive(Component, Debug)]
pub struct TeleportState {
    /// Counts up as teleports are made.
    teleport_id_counter: u32,
    /// The number of pending client teleports that have yet to receive a
    /// confirmation. Inbound client position packets should be ignored while
    /// this is nonzero.
    pending_teleports: u32,
    pub(super) synced_pos: DVec3,
    pub(super) synced_look: Look,
}

impl TeleportState {
    pub(super) fn new() -> Self {
        Self {
            teleport_id_counter: 0,
            pending_teleports: 0,
            synced_pos: DVec3::ZERO,
            synced_look: Look {
                // Client starts facing north.
                yaw: 180.0,
                pitch: 0.0,
            },
        }
    }

    pub fn teleport_id_counter(&self) -> u32 {
        self.teleport_id_counter
    }

    pub fn pending_teleports(&self) -> u32 {
        self.pending_teleports
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
                        "unexpected teleport ID for client {:?} (expected {expected}, got {got}",
                        packet.client
                    );
                    commands.entity(packet.client).remove::<Client>();
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::TELEPORT_CONFIRM_C2S)]
pub struct TeleportConfirmC2s {
    pub teleport_id: VarInt,
}

#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_POSITION_LOOK_S2C)]
pub struct PlayerPositionLookS2c {
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub flags: PlayerPositionLookFlags,
    pub teleport_id: VarInt,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct PlayerPositionLookFlags {
    pub x: bool,
    pub y: bool,
    pub z: bool,
    pub y_rot: bool,
    pub x_rot: bool,
    #[bits(3)]
    _pad: u8,
}
