//! Valence is a Minecraft server framework written in Rust.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/valence-rs/valence/main/assets/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/valence-rs/valence/main/assets/logo.svg"
)]
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
#![allow(clippy::type_complexity)] // ECS queries are often complicated.

pub use {anyhow, bevy_app, bevy_ecs, uuid, valence_nbt as nbt, valence_protocol as protocol};

#[cfg(any(test, doctest))]
mod tests;

use std::num::NonZeroU32;
use std::time::Duration;

use bevy_app::prelude::*;
use bevy_app::{ScheduleRunnerPlugin, ScheduleRunnerSettings};
use bevy_ecs::prelude::*;
use component::ComponentPlugin;

use crate::biome::BiomePlugin;
use crate::client::ClientPlugin;
use crate::dimension::DimensionPlugin;
use crate::entity::EntityPlugin;
use crate::event_loop::EventLoopPlugin;
use crate::instance::InstancePlugin;
use crate::inventory::InventoryPlugin;
use crate::player_list::PlayerListPlugin;
use crate::registry_codec::RegistryCodecPlugin;
use crate::weather::WeatherPlugin;

pub mod prelude {
    pub use bevy_app::prelude::*;
    pub use bevy_ecs::prelude::*;
    pub use biome::{Biome, BiomeId, BiomeRegistry};
    pub use client::action::*;
    pub use client::command::*;
    pub use client::interact_entity::*;
    pub use client::{
        despawn_disconnected_clients, Client, CompassPos, CursorItem, DeathLocation,
        HasRespawnScreen, HashedSeed, Ip, IsDebug, IsFlat, IsHardcore, OldView, OldViewDistance,
        OpLevel, PrevGameMode, ReducedDebugInfo, View, ViewDistance,
    };
    pub use component::*;
    pub use dimension::{DimensionType, DimensionTypeRegistry};
    pub use entity::{EntityAnimation, EntityKind, EntityManager, EntityStatus, HeadYaw};
    pub use event_loop::{EventLoopSchedule, EventLoopSet};
    pub use glam::DVec3;
    pub use instance::{Block, BlockMut, BlockRef, Chunk, Instance};
    pub use inventory::{
        Inventory, InventoryKind, InventoryWindow, InventoryWindowMut, OpenInventory,
    };
    pub use player_list::{PlayerList, PlayerListEntry};
    pub use protocol::block::{BlockState, PropName, PropValue};
    pub use protocol::ident::Ident;
    pub use protocol::item::{ItemKind, ItemStack};
    pub use protocol::text::{Color, Text, TextFormat};
    pub use uuid::Uuid;
    pub use valence_nbt::Compound;
    pub use valence_protocol::block::BlockKind;
    pub use valence_protocol::block_pos::BlockPos;
    pub use valence_protocol::ident;
    pub use valence_protocol::packet::s2c::play::particle::Particle;
    pub use view::{ChunkPos, ChunkView};

    use super::*;
    pub use crate::Server;
}

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        let settings = app
            .world
            .get_resource_or_insert_with(ServerSettings::default);

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

        app.add_system(increment_tick_counter.in_base_set(CoreSet::Last));

        // Add internal plugins.
        app.add_plugin(EventLoopPlugin)
            .add_plugin(RegistryCodecPlugin)
            .add_plugin(BiomePlugin)
            .add_plugin(DimensionPlugin)
            .add_plugin(ComponentPlugin)
            .add_plugin(ClientPlugin)
            .add_plugin(EntityPlugin)
            .add_plugin(InstancePlugin)
            .add_plugin(InventoryPlugin)
            .add_plugin(PlayerListPlugin)
            .add_plugin(WeatherPlugin);
    }
}

#[derive(Resource, Debug)]
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
    pub compression_threshold: Option<u32>,
}

impl Default for ServerSettings {
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
