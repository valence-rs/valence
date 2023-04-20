//! The weather system.
//!
//! This module contains the systems and components needed to handle
//! weather.
//!
//! # Components
//!
//! The components may be attached to clients or instances.
//!
//! - [`Rain`]: When attached, raining begin and rain level set events are
//!   emitted. When removed, the end raining event is emitted.
//! - [`Thunder`]: When attached, thunder level set event is emitted. When
//!   removed, the thunder level set to zero event is emitted.
//!
//! New joined players are handled, so that they are get weather events from
//! the instance.

use valence_core::packet::s2c::play::game_state_change::GameEventKind;
use valence_core::packet::s2c::play::GameStateChangeS2c;

use super::*;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
struct UpdateWeatherPerInstanceSet;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
struct UpdateWeatherPerClientSet;

pub(super) fn build(app: &mut App) {
    app.configure_set(
        UpdateWeatherPerInstanceSet
            .in_base_set(CoreSet::PostUpdate)
            .before(WriteUpdatePacketsToInstancesSet),
    )
    .configure_set(
        UpdateWeatherPerClientSet
            .in_base_set(CoreSet::PostUpdate)
            .before(FlushPacketsSet),
    )
    .add_systems(
        (
            handle_rain_begin_per_instance,
            handle_rain_change_per_instance,
            handle_rain_end_per_instance,
            handle_thunder_change_per_instance,
            handle_thunder_end_per_instance,
        )
            .chain()
            .in_set(UpdateWeatherPerInstanceSet)
            .before(UpdateWeatherPerClientSet),
    )
    .add_systems(
        (
            handle_rain_begin_per_client,
            handle_rain_change_per_client,
            handle_rain_end_per_client,
            handle_thunder_change_per_client,
            handle_thunder_end_per_client,
        )
            .chain()
            .in_set(UpdateWeatherPerClientSet),
    )
    .add_system(
        handle_weather_for_joined_player
            .before(UpdateWeatherPerClientSet)
            .in_base_set(CoreSet::PostUpdate),
    );
}

/// Contains the rain level.
///
/// Valid values are within `0.0..=1.0`.
#[derive(Component)]
pub struct Rain(pub f32);

/// Contains the thunder level.
///
/// Valid values are within `0.0..=1.0`.
#[derive(Component)]
pub struct Thunder(pub f32);

fn handle_weather_for_joined_player(
    mut clients: Query<(&mut Client, &Location), Added<Client>>,
    weathers: Query<(Option<&Rain>, Option<&Thunder>), With<Instance>>,
) {
    for (mut client, loc) in &mut clients {
        if let Ok((rain, thunder)) = weathers.get(loc.0) {
            if let Some(level) = rain {
                client.write_packet(&GameStateChangeS2c {
                    kind: GameEventKind::BeginRaining,
                    value: 0.0,
                });

                client.write_packet(&GameStateChangeS2c {
                    kind: GameEventKind::RainLevelChange,
                    value: level.0,
                });
            }

            if let Some(level) = thunder {
                client.write_packet(&GameStateChangeS2c {
                    kind: GameEventKind::ThunderLevelChange,
                    value: level.0,
                });
            }
        }
    }
}

fn handle_rain_begin_per_instance(mut instances: Query<&mut Instance, Added<Rain>>) {
    for mut instance in &mut instances {
        instance.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::BeginRaining,
            value: f32::default(),
        });
    }
}

fn handle_rain_change_per_instance(mut instances: Query<(&mut Instance, &Rain), Changed<Rain>>) {
    for (mut instance, rain) in &mut instances {
        instance.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::RainLevelChange,
            value: rain.0,
        });
    }
}

fn handle_rain_end_per_instance(
    mut instances: Query<&mut Instance>,
    mut removed: RemovedComponents<Rain>,
) {
    for entity in &mut removed {
        if let Ok(mut instance) = instances.get_mut(entity) {
            instance.write_packet(&GameStateChangeS2c {
                kind: GameEventKind::EndRaining,
                value: 0.0,
            });
        }
    }
}

fn handle_thunder_change_per_instance(
    mut instances: Query<(&mut Instance, &Thunder), Changed<Thunder>>,
) {
    for (mut instance, thunder) in &mut instances {
        instance.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::ThunderLevelChange,
            value: thunder.0,
        });
    }
}

fn handle_thunder_end_per_instance(
    mut instances: Query<&mut Instance>,
    mut removed: RemovedComponents<Thunder>,
) {
    for entity in &mut removed {
        if let Ok(mut instance) = instances.get_mut(entity) {
            instance.write_packet(&GameStateChangeS2c {
                kind: GameEventKind::ThunderLevelChange,
                value: 0.0,
            });
        }
    }
}

fn handle_rain_begin_per_client(mut clients: Query<&mut Client, (Added<Rain>, Without<Instance>)>) {
    for mut client in &mut clients {
        client.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::BeginRaining,
            value: 0.0,
        });
    }
}

fn handle_rain_change_per_client(
    mut clients: Query<(&mut Client, &Rain), (Changed<Rain>, Without<Instance>)>,
) {
    for (mut client, rain) in &mut clients {
        client.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::RainLevelChange,
            value: rain.0,
        });
    }
}

fn handle_rain_end_per_client(
    mut clients: Query<&mut Client>,
    mut removed: RemovedComponents<Rain>,
) {
    for entity in &mut removed {
        if let Ok(mut client) = clients.get_mut(entity) {
            client.write_packet(&GameStateChangeS2c {
                kind: GameEventKind::EndRaining,
                value: f32::default(),
            });
        }
    }
}

fn handle_thunder_change_per_client(
    mut clients: Query<(&mut Client, &Thunder), (Changed<Thunder>, Without<Instance>)>,
) {
    for (mut client, thunder) in &mut clients {
        client.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::ThunderLevelChange,
            value: thunder.0,
        });
    }
}

fn handle_thunder_end_per_client(
    mut clients: Query<&mut Client, Without<Instance>>,
    mut removed: RemovedComponents<Thunder>,
) {
    for entity in &mut removed {
        if let Ok(mut client) = clients.get_mut(entity) {
            client.write_packet(&GameStateChangeS2c {
                kind: GameEventKind::ThunderLevelChange,
                value: 0.0,
            });
        }
    }
}
