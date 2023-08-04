pub use valence_packet::packets::play::player_abilities_s2c::PlayerAbilitiesFlags;
use valence_packet::packets::play::PlayerAbilitiesS2c;

use super::*;

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
        update_player_abilities
            .in_set(UpdateClientsSet)
            .after(update_game_mode),
    );
}

fn update_player_abilities(
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
