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
    clippy::dbg_macro
)]
#![allow(
    clippy::derive_partial_eq_without_eq,
    clippy::unusual_byte_groupings,
    clippy::comparison_chain
)]

use bevy_ecs::prelude::*;
pub use {
    anyhow, async_trait, bevy_app, bevy_ecs, uuid, valence_nbt as nbt, valence_protocol as protocol,
};

pub mod biome;
pub mod client;
pub mod config;
pub mod dimension;
pub mod entity;
pub mod instance;
pub mod inventory;
pub mod math;
mod packet;
pub mod player_list;
pub mod player_textures;
pub mod server;
#[cfg(any(test, doctest))]
mod unit_test;
pub mod view;

pub mod prelude {
    pub use async_trait::async_trait;
    pub use bevy_app::App;
    pub use bevy_ecs::prelude::*;
    pub use biome::{Biome, BiomeId};
    pub use client::Client;
    pub use config::{
        AsyncCallbacks, ConnectionMode, PlayerSampleEntry, ServerListPing, ServerPlugin,
    };
    pub use dimension::{Dimension, DimensionId};
    pub use entity::{
        EntityAnimation, EntityKind, EntityStatus, McEntity, McEntityManager, TrackedData,
    };
    pub use glam::DVec3;
    pub use instance::{Block, BlockMut, BlockRef, Chunk, Instance};
    pub use inventory::{Inventory, InventoryKind, OpenInventory};
    pub use player_list::{PlayerList, PlayerListEntry};
    pub use protocol::block::{BlockState, PropName, PropValue};
    pub use protocol::ident::Ident;
    pub use protocol::text::{Color, Text, TextFormat};
    pub use protocol::types::GameMode;
    pub use protocol::username::Username;
    pub use protocol::{ident, ItemKind, ItemStack};
    pub use server::{EventLoop, NewClientInfo, Server, SharedServer};
    pub use uuid::Uuid;
    pub use valence_nbt::Compound;
    pub use valence_protocol::{BlockKind, BlockPos};
    pub use view::{ChunkPos, ChunkView};

    use super::*;
}

/// A [`Component`] for marking entities that should be despawned at the end of
/// the tick.
///
/// In Valence, some built-in components such as [`McEntity`] are not allowed to
/// be removed from the [`World`] directly. Instead, you must give the entities
/// you wish to despawn the `Despawned` component. At the end of the tick,
/// Valence will despawn all entities with this component for you.
///
/// It is legal to remove components or delete entities that Valence does not
/// know about at any time.
///
/// [`McEntity`]: entity::McEntity
#[derive(Copy, Clone, Component)]
pub struct Despawned;

/// Let's pretend that [`NULL_ENTITY`] was created by spawning an entity,
/// immediately despawning it, and then stealing its [`Entity`] ID. The user
/// doesn't need to know about this.
const NULL_ENTITY: Entity = Entity::from_bits(u64::MAX);
