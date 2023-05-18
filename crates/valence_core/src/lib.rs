#![doc = include_str!("../README.md")]
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::bare_urls,
    rustdoc::invalid_html_tags
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    unreachable_pub,
    clippy::dbg_macro
)]
#![allow(clippy::unusual_byte_groupings)]

pub mod aabb;
pub mod block_pos;
pub mod chunk_pos;
pub mod despawn;
pub mod difficulty;
pub mod direction;
pub mod game_mode;
pub mod hand;
pub mod ident;
pub mod item;
pub mod protocol;
pub mod player_textures;
pub mod property;
pub mod scratch;
pub mod text;
pub mod translation_key;
pub mod uuid;
pub mod particle;

use std::num::NonZeroU32;
use std::time::Duration;

use bevy_app::prelude::*;
use bevy_app::{ScheduleRunnerPlugin, ScheduleRunnerSettings};
use bevy_ecs::prelude::*;

use crate::despawn::despawn_marked_entities;

/// Used only by macros. Not public API.
#[doc(hidden)]
pub mod __private {
    pub use anyhow::{anyhow, bail, ensure, Context, Result};

    pub use crate::protocol::var_int::VarInt;
    pub use crate::protocol::{Decode, Encode, Packet};
}

// Needed to make proc macros work.
extern crate self as valence_core;

/// The Minecraft protocol version this library currently targets.
pub const PROTOCOL_VERSION: i32 = 762;

/// The stringified name of the Minecraft version this library currently
/// targets.
pub const MINECRAFT_VERSION: &str = "1.19.4";

/// Minecraft's standard ticks per second (TPS).
pub const DEFAULT_TPS: NonZeroU32 = match NonZeroU32::new(20) {
    Some(n) => n,
    None => unreachable!(),
};

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        let settings = app.world.get_resource_or_insert_with(CoreSettings::default);

        let compression_threshold = settings.compression_threshold;
        let tick_rate = settings.tick_rate;

        app.insert_resource(Server {
            current_tick: 0,
            compression_threshold,
        });

        let tick_period = Duration::from_secs_f64((tick_rate.get() as f64).recip());

        // Make the app loop forever at the configured TPS.
        app.insert_resource(ScheduleRunnerSettings::run_loop(tick_period))
            .add_plugin(ScheduleRunnerPlugin);

        fn increment_tick_counter(mut server: ResMut<Server>) {
            server.current_tick += 1;
        }

        app.add_systems(
            (increment_tick_counter, despawn_marked_entities).in_base_set(CoreSet::Last),
        );
    }
}

#[derive(Resource, Debug)]
pub struct CoreSettings {
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
    pub compression_threshold: Option<u32>,
}

impl Default for CoreSettings {
    fn default() -> Self {
        Self {
            tick_rate: DEFAULT_TPS,
            compression_threshold: Some(256),
        }
    }
}

/// Contains global server state accessible as a [`Resource`].
#[derive(Resource)]
pub struct Server {
    /// Incremented on every tick.
    current_tick: i64,
    compression_threshold: Option<u32>,
}

impl Server {
    /// Returns the number of ticks that have elapsed since the server began.
    pub fn current_tick(&self) -> i64 {
        self.current_tick
    }

    /// Returns the server's compression threshold.
    pub fn compression_threshold(&self) -> Option<u32> {
        self.compression_threshold
    }
}
