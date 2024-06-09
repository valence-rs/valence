#![doc = include_str!("../README.md")]

mod despawn;
mod uuid;

use std::num::NonZeroU32;
use std::time::Duration;

use bevy_app::prelude::*;
use bevy_app::ScheduleRunnerPlugin;
use bevy_ecs::prelude::*;
pub use despawn::*;
use valence_protocol::CompressionThreshold;

pub use crate::uuid::*;

/// Minecraft's standard ticks per second (TPS).
pub const DEFAULT_TPS: NonZeroU32 = match NonZeroU32::new(20) {
    Some(n) => n,
    None => unreachable!(),
};

#[derive(Clone, Resource)]
pub struct ServerSettings {
    /// The target ticks per second (TPS) of the server. This is the number of
    /// game updates that should occur in one second.
    ///
    /// On each game update (tick), the server is expected to update game logic
    /// and respond to packets from clients. Once this is complete, the server
    /// will sleep for any remaining time until a full tick duration has passed.
    ///
    /// Note that the official Minecraft client only processes packets at 20hz,
    /// so there is little benefit to a tick rate higher than the default 20.
    ///
    /// # Default Value
    ///
    /// [`DEFAULT_TPS`]
    pub tick_rate: NonZeroU32,
    /// The compression threshold to use for compressing packets. For a
    /// compression threshold of `Some(N)`, packets with encoded lengths >= `N`
    /// are compressed while all others are not. `None` disables compression
    /// completely.
    ///
    /// If the server is used behind a proxy on the same machine, you will
    /// likely want to disable compression.
    ///
    /// # Default Value
    ///
    /// Compression is enabled with an unspecified value. This value may
    /// change in future versions.
    pub compression_threshold: CompressionThreshold,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            tick_rate: DEFAULT_TPS,
            compression_threshold: CompressionThreshold(256),
        }
    }
}

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        let settings = app
            .world
            .get_resource_or_insert_with(ServerSettings::default)
            .clone();

        app.insert_resource(Server {
            current_tick: 0,
            threshold: settings.compression_threshold,
            tick_rate: settings.tick_rate,
        });

        let tick_period = Duration::from_secs_f64(f64::from(settings.tick_rate.get()).recip());

        // Make the app loop forever at the configured TPS.
        app.add_plugins(ScheduleRunnerPlugin::run_loop(tick_period));

        fn increment_tick_counter(mut server: ResMut<Server>) {
            server.current_tick += 1;
        }

        app.add_systems(Last, (increment_tick_counter, despawn_marked_entities));
    }
}

/// Contains global server state accessible as a [`Resource`].
#[derive(Resource)]
pub struct Server {
    /// Incremented on every tick.
    current_tick: i64,
    threshold: CompressionThreshold,
    tick_rate: NonZeroU32,
}

impl Server {
    /// Returns the number of ticks that have elapsed since the server began.
    pub fn current_tick(&self) -> i64 {
        self.current_tick
    }

    /// Returns the server's [compression
    /// threshold](ServerSettings::compression_threshold).
    pub fn compression_threshold(&self) -> CompressionThreshold {
        self.threshold
    }

    // Returns the server's [tick rate](ServerPlugin::tick_rate).
    pub fn tick_rate(&self) -> NonZeroU32 {
        self.tick_rate
    }
}
