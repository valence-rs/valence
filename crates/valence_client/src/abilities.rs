pub use valence_packet::packets::play::player_abilities_s2c::PlayerAbilitiesFlags;
use valence_packet::packets::play::{PlayerAbilitiesS2c, UpdatePlayerAbilitiesC2s};

use super::*;
use crate::event_loop::{EventLoopPreUpdate, PacketEvent};

/// [`Component`] that stores the player's flying speed ability.
///
/// [`Default`] value: `0.05`.
#[derive(Component)]
pub struct FlyingSpeed(pub f32);

impl Default for FlyingSpeed {
    fn default() -> Self {
        Self(0.05)
    }
}

/// [`Component`] that stores the player's field of view modifier ability.
/// The lower the value, the higher the field of view.
///
/// [`Default`] value: `0.1`.
#[derive(Component)]
pub struct FovModifier(pub f32);

impl Default for FovModifier {
    fn default() -> Self {
        Self(0.1)
    }
}

pub(super) fn build(app: &mut App) {
    app.add_systems(
        PostUpdate,
        (
            update_client_player_abilities,
            update_player_abilities.before(update_client_player_abilities),
        )
            .in_set(UpdateClientsSet)
            .after(update_game_mode),
    )
    .add_systems(EventLoopPreUpdate, update_server_player_abilities);
}

fn update_client_player_abilities(
    mut clients_query: Query<
        (
            &mut Client,
            &PlayerAbilitiesFlags,
            &FlyingSpeed,
            &FovModifier,
        ),
        Or<(
            Changed<PlayerAbilitiesFlags>,
            Changed<FlyingSpeed>,
            Changed<FovModifier>,
        )>,
    >,
) {
    for (mut client, flags, flying_speed, fov_modifier) in clients_query.iter_mut() {
        client.write_packet(&PlayerAbilitiesS2c {
            flags: *flags,
            flying_speed: flying_speed.0,
            fov_modifier: fov_modifier.0,
        })
    }
}

fn update_player_abilities(
    mut client_query: Query<(&mut PlayerAbilitiesFlags, &GameMode), Changed<GameMode>>,
) {
    for (mut flags, gamemode) in client_query.iter_mut() {
        match gamemode {
            GameMode::Creative => {
                flags.set_invulnerable(true);
                flags.set_allow_flying(true);
                flags.set_instant_break(true);
            }
            GameMode::Spectator => {
                flags.set_invulnerable(true);
                flags.set_allow_flying(true);
                flags.set_instant_break(false);
                flags.set_flying(true);
            }
            _ => {
                flags.set_invulnerable(false);
                flags.set_allow_flying(false);
                flags.set_instant_break(false);
            }
        }
    }
}

fn update_server_player_abilities(
    mut packet_events: EventReader<PacketEvent>,
    mut client_query: Query<&mut PlayerAbilitiesFlags>,
) {
    for packets in packet_events.iter() {
        if let Some(pkt) = packets.decode::<UpdatePlayerAbilitiesC2s>() {
            if let Ok(mut flags) = client_query.get_mut(packets.client) {
                flags.set_flying(UpdatePlayerAbilitiesC2s::StartFlying.eq(&pkt));
                flags.bypass_change_detection();
            }
        }
    }
}
