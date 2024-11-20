use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use tracing::warn;
use valence_entity::{Look, Position};
use valence_math::DVec3;
use valence_protocol::packets::play::player_position_s2c::TeleportRelativeFlags;
use valence_protocol::packets::play::{AcceptTeleportationC2s, PlayerPositionS2c};
use valence_protocol::WritePacket;

use crate::client::{update_view_and_layers, Client, UpdateClientsSet};
use crate::event_loop::{EventLoopPreUpdate, PacketEvent};
use crate::spawn::update_respawn_position;

pub struct TeleportPlugin;

impl Plugin for TeleportPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            teleport
                .after(update_view_and_layers)
                .before(update_respawn_position)
                .in_set(UpdateClientsSet),
        )
        .add_systems(EventLoopPreUpdate, handle_teleport_confirmations);
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
    pub(super) synced_pos: DVec3,
    pub(super) synced_look: Look,
}

impl TeleportState {
    pub(super) fn new() -> Self {
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

            let flags = TeleportRelativeFlags::new()
                .with_x(!changed_pos)
                .with_y(!changed_pos)
                .with_z(!changed_pos)
                .with_y_rot(!changed_yaw)
                .with_x_rot(!changed_pitch);

            client.write_packet(&PlayerPositionS2c {
                position: if changed_pos { pos.0 } else { DVec3::ZERO },
                // FIXME: add missing velocity
                velocity: if changed_pos { pos.0 } else { DVec3::ZERO },
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
    for packet in packets.read() {
        if let Some(pkt) = packet.decode::<AcceptTeleportationC2s>() {
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
