use std::ops::Range;

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::SystemConfigs;
use valence_protocol::packet::s2c::play::game_state_change::GameEventKind;
use valence_protocol::packet::s2c::play::GameStateChangeS2c;

use crate::packet::WritePacket;
use crate::prelude::*;

pub const WEATHER_LEVEL_RANGE: Range<f32> = 0_f32..1_f32;

/// The weather state representation.
#[derive(Component)]
pub struct Weather {
    /// Contains the rain level.
    /// Valid value is a value within the [WEATHER_LEVEL_RANGE] range.
    /// Invalid value would be clamped.
    ///
    /// The [`None`] value means no rain level.
    pub rain: Option<f32>,
    /// Contains the thunder level.
    /// Valid value is a value within the [WEATHER_LEVEL_RANGE] range.
    /// Invalid value would be clamped.
    ///
    /// The [`None`] value means no thunder level.
    pub thunder: Option<f32>,
}

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
            value: level.clamp(WEATHER_LEVEL_RANGE.start, WEATHER_LEVEL_RANGE.end),
        });
    }

    /// Sends the set thunder level event to all players in the instance.
    fn set_thunder_level(&mut self, level: f32) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::ThunderLevelChange,
            value: level.clamp(WEATHER_LEVEL_RANGE.start, WEATHER_LEVEL_RANGE.end),
        });
    }

    /// Sends weather level events to all players in the instance.
    fn set_weather(&mut self, weather: &Weather) {
        if let Some(rain_level) = weather.rain {
            self.set_rain_level(rain_level)
        }

        if let Some(thunder_level) = weather.thunder {
            self.set_thunder_level(thunder_level)
        }
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
            value: level.clamp(WEATHER_LEVEL_RANGE.start, WEATHER_LEVEL_RANGE.end),
        });
    }

    /// Sends the set thunder level event to the client.
    fn set_thunder_level(&mut self, level: f32) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::ThunderLevelChange,
            value: level.clamp(WEATHER_LEVEL_RANGE.start, WEATHER_LEVEL_RANGE.end),
        });
    }

    /// Sends weather level events to the client.
    fn set_weather(&mut self, weather: &Weather) {
        if let Some(rain_level) = weather.rain {
            self.set_rain_level(rain_level)
        }

        if let Some(thunder_level) = weather.thunder {
            self.set_thunder_level(thunder_level)
        }
    }
}

fn handle_weather_begin_per_instance(mut query: Query<&mut Instance, Added<Weather>>) {
    query.for_each_mut(|mut instance| {
        instance.begin_raining();
    });
}

fn handle_weather_end_per_instance(
    mut query: Query<&mut Instance>,
    mut removed: RemovedComponents<Weather>,
) {
    removed.iter().for_each(|entity| {
        if let Ok(mut instance) = query.get_mut(entity) {
            instance.end_raining();
        }
    })
}

fn handle_weather_change_per_instance(
    mut query: Query<(&mut Instance, &Weather), Changed<Weather>>,
) {
    query.for_each_mut(|(mut instance, weather)| {
        instance.set_weather(weather);
    });
}

fn handle_weather_for_joined_player(
    mut clients: Query<&mut Client, Added<Client>>,
    weathers: Query<&Weather, With<Instance>>,
) {
    clients.for_each_mut(|mut client| {
        if let Ok(weather) = weathers.get_single() {
            client.begin_raining();
            client.set_weather(weather);
        }
    })
}

fn handle_weather_begin_per_client(mut query: Query<&mut Client, Added<Weather>>) {
    query.for_each_mut(|mut client| {
        client.begin_raining();
    });
}

fn handle_weather_end_per_client(
    mut query: Query<&mut Client>,
    mut removed: RemovedComponents<Weather>,
) {
    removed.iter().for_each(|entity| {
        if let Ok(mut client) = query.get_mut(entity) {
            client.end_raining();
        }
    })
}

fn handle_weather_change_per_client(mut query: Query<(&mut Client, &Weather), Changed<Weather>>) {
    query.for_each_mut(|(mut client, weather)| {
        client.set_weather(weather);
    });
}

pub(crate) fn update_weather() -> SystemConfigs {
    (
        handle_weather_for_joined_player,
        handle_weather_begin_per_instance.before(handle_weather_change_per_instance),
        handle_weather_change_per_instance.before(handle_weather_end_per_instance),
        handle_weather_end_per_instance,
        handle_weather_begin_per_client.before(handle_weather_change_per_client),
        handle_weather_change_per_client.before(handle_weather_end_per_client),
        handle_weather_end_per_client,
    )
        .into_configs()
}

#[cfg(test)]
mod test {
    use anyhow::Ok;
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
            assert_eq!(pkt.value, 0.5f32);
        }

        if let S2cPlayPacket::GameStateChangeS2c(pkt) = sent_packets[3] {
            assert_eq!(pkt.value, WEATHER_LEVEL_RANGE.end);
        }

        if let S2cPlayPacket::GameStateChangeS2c(pkt) = sent_packets[4] {
            assert_eq!(pkt.value, WEATHER_LEVEL_RANGE.start);
        }
    }

    #[test]
    fn test_weather_instance() -> anyhow::Result<()> {
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

        // Insert a weather component to the instance.
        app.world.entity_mut(instance_ent).insert(Weather {
            rain: Some(0.5f32),
            thunder: Some(0.5f32),
        });

        // Handle weather event packets.
        for _ in 0..4 {
            app.update();
        }

        // Alter a weather component of the instance.
        app.world.entity_mut(instance_ent).insert(Weather {
            // Invalid values to assert they are clamped.
            rain: Some(WEATHER_LEVEL_RANGE.end + 1_f32),
            thunder: Some(WEATHER_LEVEL_RANGE.start - 1_f32),
        });
        app.update();

        // Remove the weather component from the instance.
        app.world.entity_mut(instance_ent).remove::<Weather>();
        app.update();

        // Make assertions.
        let sent_packets = client_helper.collect_sent()?;

        assert_weather_packets(sent_packets);

        Ok(())
    }

    #[test]
    fn test_weather_client() -> anyhow::Result<()> {
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

        // Insert a weather component to the client.
        app.world.entity_mut(client_ent).insert(Weather {
            rain: Some(0.5f32),
            thunder: Some(0.5f32),
        });

        // Handle weather event packets.
        for _ in 0..4 {
            app.update();
        }

        // Alter a weather component of the client.
        app.world.entity_mut(client_ent).insert(Weather {
            // Invalid values to assert they are clamped.
            rain: Some(WEATHER_LEVEL_RANGE.end + 1_f32),
            thunder: Some(WEATHER_LEVEL_RANGE.start - 1_f32),
        });
        app.update();

        // Remove the weather component from the client.
        app.world.entity_mut(client_ent).remove::<Weather>();
        app.update();

        // Make assertions.
        let sent_packets = client_helper.collect_sent()?;

        assert_weather_packets(sent_packets);

        Ok(())
    }
}
