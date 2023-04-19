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
