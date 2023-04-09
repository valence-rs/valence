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

use std::ops::Range;

use bevy_ecs::prelude::*;
use valence_protocol::packet::s2c::play::game_state_change::GameEventKind;
use valence_protocol::packet::s2c::play::GameStateChangeS2c;

use crate::client::FlushPacketsSet;
use crate::instance::WriteUpdatePacketsToInstancesSet;
use crate::packet::WritePacket;
use crate::prelude::*;

pub const WEATHER_LEVEL: Range<f32> = 0_f32..1_f32;

/// Contains the rain level.
/// Valid value is a value within the [WEATHER_LEVEL] range.
/// Invalid value would be clamped.
#[derive(Component)]
pub struct Rain(pub f32);

/// Contains the thunder level.
/// Valid value is a value within the [WEATHER_LEVEL] range.
/// Invalid value would be clamped.
#[derive(Component)]
pub struct Thunder(pub f32);

impl Instance {
    /// Sends the begin rain event to all players in the instance.
    fn begin_raining(&mut self) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::BeginRaining,
            value: f32::default(),
        });
    }

    /// Sends the end rain event to all players in the instance.
    fn end_raining(&mut self) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::EndRaining,
            value: f32::default(),
        });
    }

    /// Sends the set rain level event to all players in the instance.
    fn set_rain_level(&mut self, level: f32) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::RainLevelChange,
            value: level.clamp(WEATHER_LEVEL.start, WEATHER_LEVEL.end),
        });
    }

    /// Sends the set thunder level event to all players in the instance.
    fn set_thunder_level(&mut self, level: f32) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::ThunderLevelChange,
            value: level.clamp(WEATHER_LEVEL.start, WEATHER_LEVEL.end),
        });
    }
}

impl Client {
    /// Sends the begin rain event to the client.
    fn begin_raining(&mut self) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::BeginRaining,
            value: f32::default(),
        });
    }

    /// Sends the end rain event to the client.
    fn end_raining(&mut self) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::EndRaining,
            value: f32::default(),
        });
    }

    /// Sends the set rain level event to the client.
    fn set_rain_level(&mut self, level: f32) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::RainLevelChange,
            value: level.clamp(WEATHER_LEVEL.start, WEATHER_LEVEL.end),
        });
    }

    /// Sends the set thunder level event to the client.
    fn set_thunder_level(&mut self, level: f32) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::ThunderLevelChange,
            value: level.clamp(WEATHER_LEVEL.start, WEATHER_LEVEL.end),
        });
    }
}

fn handle_weather_for_joined_player(
    mut clients: Query<(&mut Client, &Location), Added<Client>>,
    weathers: Query<(Option<&Rain>, Option<&Thunder>), With<Instance>>,
) {
    clients.for_each_mut(|(mut client, loc)| {
        if let Ok((rain, thunder)) = weathers.get(loc.0) {
            if let Some(level) = rain {
                client.begin_raining();
                client.set_rain_level(level.0);
            }

            if let Some(level) = thunder {
                client.set_thunder_level(level.0);
            }
        }
    })
}

fn handle_rain_begin_per_instance(mut query: Query<&mut Instance, Added<Rain>>) {
    query.for_each_mut(|mut instance| {
        instance.begin_raining();
    });
}

fn handle_rain_change_per_instance(mut query: Query<(&mut Instance, &Rain), Changed<Rain>>) {
    query.for_each_mut(|(mut instance, rain)| instance.set_rain_level(rain.0));
}

fn handle_rain_end_per_instance(
    mut query: Query<&mut Instance>,
    mut removed: RemovedComponents<Rain>,
) {
    removed.iter().for_each(|entity| {
        if let Ok(mut instance) = query.get_mut(entity) {
            instance.end_raining();
        }
    })
}

fn handle_thunder_change_per_instance(
    mut query: Query<(&mut Instance, &Thunder), Changed<Thunder>>,
) {
    query.for_each_mut(|(mut instance, thunder)| instance.set_thunder_level(thunder.0));
}

fn handle_thunder_end_per_instance(
    mut query: Query<&mut Instance>,
    mut removed: RemovedComponents<Thunder>,
) {
    removed.iter().for_each(|entity| {
        if let Ok(mut instance) = query.get_mut(entity) {
            instance.set_thunder_level(WEATHER_LEVEL.start);
        }
    })
}

fn handle_rain_begin_per_client(mut query: Query<&mut Client, (Added<Rain>, Without<Instance>)>) {
    query.for_each_mut(|mut client| {
        client.begin_raining();
    });
}

fn handle_rain_change_per_client(
    mut query: Query<(&mut Client, &Rain), (Changed<Rain>, Without<Instance>)>,
) {
    query.for_each_mut(|(mut client, rain)| {
        client.set_rain_level(rain.0);
    });
}

fn handle_rain_end_per_client(mut query: Query<&mut Client>, mut removed: RemovedComponents<Rain>) {
    removed.iter().for_each(|entity| {
        if let Ok(mut client) = query.get_mut(entity) {
            client.end_raining();
        }
    })
}

fn handle_thunder_change_per_client(
    mut query: Query<(&mut Client, &Thunder), (Changed<Thunder>, Without<Instance>)>,
) {
    query.for_each_mut(|(mut client, thunder)| {
        client.set_thunder_level(thunder.0);
    });
}

fn handle_thunder_end_per_client(
    mut query: Query<&mut Client, Without<Instance>>,
    mut removed: RemovedComponents<Thunder>,
) {
    removed.iter().for_each(|entity| {
        if let Ok(mut client) = query.get_mut(entity) {
            client.set_thunder_level(WEATHER_LEVEL.start);
        }
    })
}

pub(crate) struct WeatherPlugin;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct UpdateWeatherPerInstanceSet;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct UpdateWeatherPerClientSet;

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
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
}

#[cfg(test)]
mod test {
    use bevy_app::App;
    use valence_protocol::packet::S2cPlayPacket;

    use super::*;
    use crate::unit_test::util::scenario_single_client;
    use crate::{assert_packet_count, assert_packet_order};

    fn assert_weather_packets(sent_packets: Vec<S2cPlayPacket>) {
        assert_packet_count!(sent_packets, 6, S2cPlayPacket::GameStateChangeS2c(_));

        assert_packet_order!(
            sent_packets,
            S2cPlayPacket::GameStateChangeS2c(GameStateChangeS2c {
                kind: GameEventKind::BeginRaining,
                value: _
            }),
            S2cPlayPacket::GameStateChangeS2c(GameStateChangeS2c {
                kind: GameEventKind::RainLevelChange,
                value: _
            }),
            S2cPlayPacket::GameStateChangeS2c(GameStateChangeS2c {
                kind: GameEventKind::ThunderLevelChange,
                value: _
            }),
            S2cPlayPacket::GameStateChangeS2c(GameStateChangeS2c {
                kind: GameEventKind::EndRaining,
                value: _
            })
        );

        if let S2cPlayPacket::GameStateChangeS2c(pkt) = sent_packets[1] {
            assert_eq!(pkt.value, 0.5f32);
        }

        if let S2cPlayPacket::GameStateChangeS2c(pkt) = sent_packets[2] {
            assert_eq!(pkt.value, WEATHER_LEVEL.end);
        }

        if let S2cPlayPacket::GameStateChangeS2c(pkt) = sent_packets[3] {
            assert_eq!(pkt.value, 0.5f32);
        }

        if let S2cPlayPacket::GameStateChangeS2c(pkt) = sent_packets[4] {
            assert_eq!(pkt.value, WEATHER_LEVEL.end);
        }
    }

    #[test]
    fn test_weather_instance() {
        let mut app = App::new();
        let (_, mut client_helper) = scenario_single_client(&mut app);

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        // Get the instance entity.
        let instance_ent = app
            .world
            .iter_entities()
            .find(|e| e.contains::<Instance>())
            .expect("could not find instance")
            .id();

        // Insert a rain component to the instance.
        app.world.entity_mut(instance_ent).insert(Rain(0.5f32));
        for _ in 0..2 {
            app.update();
        }

        // Alter a rain component of the instance.
        app.world.entity_mut(instance_ent).insert(Rain(
            // Invalid value to assert it is clamped.
            WEATHER_LEVEL.end + 1_f32,
        ));
        app.update();

        // Insert a thunder component to the instance.
        app.world.entity_mut(instance_ent).insert(Thunder(0.5f32));
        app.update();

        // Alter a thunder component of the instance.
        app.world.entity_mut(instance_ent).insert(Thunder(
            // Invalid value to assert it is clamped.
            WEATHER_LEVEL.end + 1_f32,
        ));
        app.update();

        // Remove the rain component from the instance.
        app.world.entity_mut(instance_ent).remove::<Rain>();
        for _ in 0..2 {
            app.update();
        }

        // Make assertions.
        let sent_packets = client_helper.collect_sent();

        assert_weather_packets(sent_packets);
    }

    #[test]
    fn test_weather_client() {
        let mut app = App::new();
        let (_, mut client_helper) = scenario_single_client(&mut app);

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        // Get the client entity.
        let client_ent = app
            .world
            .iter_entities()
            .find(|e| e.contains::<Client>())
            .expect("could not find client")
            .id();

        // Insert a rain component to the client.
        app.world.entity_mut(client_ent).insert(Rain(0.5f32));
        for _ in 0..2 {
            app.update();
        }

        // Alter a rain component of the client.
        app.world.entity_mut(client_ent).insert(Rain(
            // Invalid value to assert it is clamped.
            WEATHER_LEVEL.end + 1_f32,
        ));
        app.update();

        // Insert a thunder component to the client.
        app.world.entity_mut(client_ent).insert(Thunder(0.5f32));
        app.update();

        // Alter a thunder component of the client.
        app.world.entity_mut(client_ent).insert(Thunder(
            // Invalid value to assert it is clamped.
            WEATHER_LEVEL.end + 1_f32,
        ));
        app.update();

        // Remove the rain component from the client.
        app.world.entity_mut(client_ent).remove::<Rain>();
        for _ in 0..2 {
            app.update();
        }

        // Make assertions.
        let sent_packets = client_helper.collect_sent();

        assert_weather_packets(sent_packets);
    }
}
