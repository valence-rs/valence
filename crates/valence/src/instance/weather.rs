use bevy_ecs::prelude::*;
use valence_protocol::packet::s2c::play::game_state_change::GameEventKind;
use valence_protocol::packet::s2c::play::GameStateChangeS2c;

use super::Instance;
use crate::client::Client;

pub const WEATHER_LEVEL_MIN: f32 = 0_f32;
pub const WEATHER_LEVEL_MAX: f32 = 1_f32;

/// The weather state representation.
#[derive(Component)]
pub struct Weather {
    /// Contains the rain level.
    /// Should be between [`WEATHER_LEVEL_MIN`] and [`WEATHER_LEVEL_MAX`].
    ///
    /// The [`None`] value means no raining event.
    pub rain: Option<f32>,
    /// Contains the thunder level.
    /// Should be between [`WEATHER_LEVEL_MIN`] and [`WEATHER_LEVEL_MAX`].
    ///
    /// The [`None`] value means no thunder event.
    pub thunder: Option<f32>,
}

impl Instance {
    /// Sends the begin rain event to all players in the instance.
    pub fn begin_raining(&mut self) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::BeginRaining,
            value: f32::default(),
        });
    }

    /// Sends the end rain event to all players in the instance.
    pub fn end_raining(&mut self) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::EndRaining,
            value: f32::default(),
        });
    }

    /// Sends the set rain level event to all players in the instance.
    pub fn set_rain_level(&mut self, level: f32) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::RainLevelChange,
            value: level.clamp(WEATHER_LEVEL_MIN, WEATHER_LEVEL_MAX),
        });
    }

    /// Sends the set thunder level event to all players in the instance.
    pub fn set_thunder_level(&mut self, level: f32) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::ThunderLevelChange,
            value: level.clamp(WEATHER_LEVEL_MIN, WEATHER_LEVEL_MAX),
        });
    }
}

impl Client {
    /// Sends the begin rain event to the client.
    pub fn begin_raining(&mut self) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::BeginRaining,
            value: f32::default(),
        });
    }

    /// Sends the end rain event to the client.
    pub fn end_raining(&mut self) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::EndRaining,
            value: f32::default(),
        });
    }

    /// Sends the set rain level event to the client.
    pub fn set_rain_level(&mut self, level: f32) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::RainLevelChange,
            value: level.clamp(WEATHER_LEVEL_MIN, WEATHER_LEVEL_MAX),
        });
    }

    /// Sends the set thunder level event to the client.
    pub fn set_thunder_level(&mut self, level: f32) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::ThunderLevelChange,
            value: level.clamp(WEATHER_LEVEL_MIN, WEATHER_LEVEL_MAX),
        });
    }
}
