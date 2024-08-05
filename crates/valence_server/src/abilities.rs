use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use derive_more::{Deref, DerefMut};
pub use valence_protocol::packets::play::player_abilities_s2c::PlayerAbilitiesFlags;
use valence_protocol::packets::play::{PlayerAbilitiesS2c, UpdatePlayerAbilitiesC2s};
use valence_protocol::{GameMode, WritePacket};

use crate::client::{update_game_mode, Client, UpdateClientsSet};
use crate::event_loop::{EventLoopPreUpdate, PacketEvent};

/// [`Component`] that stores the player's flying speed ability.
///
/// [`Default`] value: `0.05`.
#[derive(Component, Deref, DerefMut)]
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
#[derive(Component, Deref, DerefMut)]
pub struct FovModifier(pub f32);

impl Default for FovModifier {
    fn default() -> Self {
        Self(0.1)
    }
}

/// Send if the client sends [`UpdatePlayerAbilitiesC2s::StartFlying`]
#[derive(Event)]
pub struct PlayerStartFlyingEvent {
    pub client: Entity,
}

/// Send if the client sends [`UpdatePlayerAbilitiesC2s::StopFlying`]
#[derive(Event)]
pub struct PlayerStopFlyingEvent {
    pub client: Entity,
}

/// Order of execution:
/// 1. `update_game_mode`: Watch [`GameMode`] changes => Send
///    `GameStateChangeS2c` to update the client's gamemode
///
/// 2. `update_client_player_abilities`: Watch [`PlayerAbilitiesFlags`],
///    [`FlyingSpeed`] and [`FovModifier`] changes => Send
///    [`PlayerAbilitiesS2c`] to update the client's abilities
///
/// 3. `update_player_abilities`: Watch [`GameMode`] changes => Update
///    [`PlayerAbilitiesFlags`] according to the [`GameMode`]
///
/// 4. `update_server_player_abilities`: Watch [`UpdatePlayerAbilitiesC2s`]
///    packets => Update [`PlayerAbilitiesFlags`] according to the packet
pub struct AbilitiesPlugin;

impl Plugin for AbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PlayerStartFlyingEvent>()
            .add_event::<PlayerStopFlyingEvent>()
            .add_systems(
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
    for (mut client, flags, flying_speed, fov_modifier) in &mut clients_query {
        client.write_packet(&PlayerAbilitiesS2c {
            flags: *flags,
            flying_speed: flying_speed.0,
            fov_modifier: fov_modifier.0,
        })
    }
}

/// /!\ This system does not trigger change detection on
/// [`PlayerAbilitiesFlags`]
fn update_player_abilities(
    mut player_start_flying_event_writer: EventWriter<PlayerStartFlyingEvent>,
    mut player_stop_flying_event_writer: EventWriter<PlayerStopFlyingEvent>,
    mut client_query: Query<(Entity, &mut PlayerAbilitiesFlags, &GameMode), Changed<GameMode>>,
) {
    for (entity, mut mut_flags, gamemode) in &mut client_query {
        let flags = mut_flags.bypass_change_detection();
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
                player_start_flying_event_writer.send(PlayerStartFlyingEvent { client: entity });
            }
            GameMode::Survival => {
                flags.set_invulnerable(false);
                flags.set_allow_flying(false);
                flags.set_instant_break(false);
                flags.set_flying(false);
                player_stop_flying_event_writer.send(PlayerStopFlyingEvent { client: entity });
            }
            GameMode::Adventure => {
                flags.set_invulnerable(false);
                flags.set_allow_flying(false);
                flags.set_instant_break(false);
                flags.set_flying(false);
                player_stop_flying_event_writer.send(PlayerStopFlyingEvent { client: entity });
            }
        }
    }
}

/// /!\ This system does not trigger change detection on
/// [`PlayerAbilitiesFlags`]
fn update_server_player_abilities(
    mut packet_events: EventReader<PacketEvent>,
    mut player_start_flying_event_writer: EventWriter<PlayerStartFlyingEvent>,
    mut player_stop_flying_event_writer: EventWriter<PlayerStopFlyingEvent>,
    mut client_query: Query<&mut PlayerAbilitiesFlags>,
) {
    for packets in packet_events.read() {
        if let Some(pkt) = packets.decode::<UpdatePlayerAbilitiesC2s>() {
            if let Ok(mut mut_flags) = client_query.get_mut(packets.client) {
                let flags = mut_flags.bypass_change_detection();
                match pkt {
                    UpdatePlayerAbilitiesC2s::StartFlying => {
                        flags.set_flying(true);
                        player_start_flying_event_writer.send(PlayerStartFlyingEvent {
                            client: packets.client,
                        });
                    }
                    UpdatePlayerAbilitiesC2s::StopFlying => {
                        flags.set_flying(false);
                        player_stop_flying_event_writer.send(PlayerStopFlyingEvent {
                            client: packets.client,
                        });
                    }
                }
            }
        }
    }
}
