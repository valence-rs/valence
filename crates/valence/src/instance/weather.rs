use bevy_ecs::prelude::*;

pub const WEATHER_LEVEL_MIN: f32 = 0_f32;
pub const WEATHER_LEVEL_MAX: f32 = 1_f32;

/// The weather state representation.
#[derive(Component)]
pub struct Weather {
    /// Contains the raining level.
    /// Should be between [`WEATHER_LEVEL_MIN`] and [`WEATHER_LEVEL_MAX`].
    ///
    /// The [`None`] value means no raining event.
    pub raining: Option<f32>,
    /// Contains the thunder level.
    /// Should be between [`WEATHER_LEVEL_MIN`] and [`WEATHER_LEVEL_MAX`].
    ///
    /// The [`None`] value means no thunder event.
    pub thunder: Option<f32>,
}
