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
#![allow(clippy::type_complexity)] // ECS queries are often complicated.

pub use {
    anyhow, async_trait, bevy_app, bevy_ecs, uuid, valence_nbt as nbt, valence_protocol as protocol,
};

pub mod biome;
pub mod client;
pub mod component;
pub mod config;
pub mod dimension;
pub mod entity;
pub mod event_loop;
pub mod instance;
pub mod inventory;
pub mod packet;
pub mod player_list;
pub mod player_textures;
pub mod registry_codec;
pub mod server;
#[cfg(any(test, doctest))]
mod unit_test;
pub mod util;
pub mod view;
pub mod weather;

pub mod prelude {
    pub use async_trait::async_trait;
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
    pub use config::{
        AsyncCallbacks, ConnectionMode, PlayerSampleEntry, ServerListPing, ServerPlugin,
    };
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
    pub use server::{NewClientInfo, Server, SharedServer};
    pub use uuid::Uuid;
    pub use valence_nbt::Compound;
    pub use valence_protocol::block::BlockKind;
    pub use valence_protocol::block_pos::BlockPos;
    pub use valence_protocol::ident;
    pub use valence_protocol::packet::s2c::play::particle::Particle;
    pub use view::{ChunkPos, ChunkView};

    use super::*;
}
