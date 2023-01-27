//! <img src="https://raw.githubusercontent.com/rj00a/valence/main/assets/logo-full.svg" width="400">
//!
//! ---
//!
//! A Rust framework for building Minecraft servers.
//!
//! At a high level, a Valence [`Server`] is a collection of [`Clients`],
//! [`Entities`], and [`Worlds`]. When a client connects to the server they are
//! added to the collection of `Clients`. After connecting, clients should
//! be assigned to a [`World`] where they can interact with the entities
//! and [`Chunks`] that are a part of it.
//!
//! The Valence documentation assumes some familiarity with Minecraft and its
//! mechanics. See the [Minecraft Wiki] for general information and [wiki.vg]
//! for protocol documentation.
//!
//! For more information, see the repository [README].
//!
//! [Minecraft Wiki]: https://minecraft.fandom.com/wiki/Minecraft_Wiki
//! [wiki.vg]: https://wiki.vg/Main_Page
//! [README]: https://github.com/rj00a/valence
//!
//! # Logging
//!
//! Valence uses the [log] crate to report errors and other information. You may
//! want to use a logging implementation such as [env_logger] to see these
//! messages.
//!
//! [log]: https://docs.rs/log/latest/log/
//! [env_logger]: https://docs.rs/env_logger/latest/env_logger/
//!
//! # An Important Note on [`mem::swap`]
//!
//! In Valence, many types are owned by the library but given out as mutable
//! references for the user to modify. Examples of such types include [`World`],
//! [`LoadedChunk`], [`Entity`], and [`Client`].
//!
//! **You must not call [`mem::swap`] on these references (or any other
//! function that would move their location in memory).** Doing so breaks
//! invariants within the library and the resulting behavior is safe but
//! unspecified. You can think of these types as being [pinned](std::pin).
//!
//! Preventing this illegal behavior using Rust's type system was considered too
//! cumbersome, so this note has been left here instead.
//!
//! [`mem::swap`]: std::mem::swap
//!
//! # Examples
//!
//! See the [examples] directory in the source repository.
//!
//! [examples]: https://github.com/rj00a/valence/tree/main/examples
//!
//! [`Server`]: crate::server::Server
//! [`Clients`]: crate::client::Clients
//! [`Entities`]: crate::entity::Entities
//! [`Worlds`]: crate::world::Worlds
//! [`World`]: crate::world::World
//! [`Chunks`]: crate::chunk::Chunks
//! [`LoadedChunk`]: crate::chunk::LoadedChunk
//! [`Entity`]: crate::entity::Entity
//! [`Client`]: crate::client::Client

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/valence-rs/valence/main/assets/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/valence-rs/valence/main/assets/logo.svg"
)]
#![forbid(unsafe_code)]
// Deny these to make CI checks fail. TODO: invalid_html_tags
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::bare_urls
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

/// Used on [`Config`](config::Config) to allow for async methods in traits.
///
/// For more information see the [async_trait] crate.
///
/// [async_trait]: https://docs.rs/async-trait/latest/async_trait/
pub use async_trait::async_trait;
#[doc(inline)]
pub use server::start_server;
pub use valence_protocol as protocol;
#[doc(inline)]
pub use {uuid, valence_nbt as nbt, vek};

pub mod biome;
pub mod chunk;
pub mod client;
pub mod config;
pub mod dimension;
pub mod entity;
pub mod inventory;
mod packet;
pub mod player_list;
pub mod player_textures;
pub mod server;
mod slab;
mod slab_rc;
mod slab_versioned;
pub mod util;
pub mod world;

/// Use `valence::prelude::*` to import the most commonly used items from the
/// library.
pub mod prelude {
    pub use biome::{Biome, BiomeId};
    pub use chunk::{Chunk, ChunkPos, Chunks, LoadedChunk, UnloadedChunk};
    pub use client::{Client, ClientEvent, ClientId, Clients};
    pub use config::{Config, ConnectionMode, PlayerSampleEntry, ServerListPing};
    pub use dimension::{Dimension, DimensionId};
    pub use entity::{Entities, Entity, EntityEvent, EntityId, EntityKind, TrackedData};
    pub use inventory::{Inventories, Inventory, InventoryId};
    pub use player_list::{PlayerList, PlayerListEntry, PlayerListId, PlayerLists};
    pub use server::{NewClientData, Server, SharedServer, ShutdownResult};
    pub use util::{from_yaw_and_pitch, to_yaw_and_pitch};
    pub use uuid::Uuid;
    pub use valence_nbt::Compound;
    pub use valence_protocol::block::{PropName, PropValue};
    pub use valence_protocol::entity_meta::Pose;
    pub use valence_protocol::ident::IdentError;
    pub use valence_protocol::packets::s2c::particle::Particle;
    pub use valence_protocol::packets::s2c::play::SetTitleAnimationTimes;
    pub use valence_protocol::text::Color;
    pub use valence_protocol::types::{GameMode, Hand, SoundCategory};
    pub use valence_protocol::{
        ident, translation_key, BlockKind, BlockPos, BlockState, Ident, InventoryKind, ItemKind,
        ItemStack, Text, TextFormat, Username, MINECRAFT_VERSION, PROTOCOL_VERSION,
    };
    pub use vek::{Aabb, Mat2, Mat3, Mat4, Vec2, Vec3, Vec4};
    pub use world::{World, WorldId, Worlds};

    use super::*;
    pub use crate::{async_trait, nbt, vek, Ticks, STANDARD_TPS};
}

/// The namespace for this library used internally for
/// [identifiers](valence_protocol::ident::Ident).
const LIBRARY_NAMESPACE: &str = "valence";

/// The most recent version of the [Velocity] proxy which has been tested to
/// work with Valence. The elements of the tuple are (major, minor, patch)
/// version numbers.
///
/// See [`Config::connection_mode`] to configure the proxy used with Valence.
///
/// [Velocity]: https://velocitypowered.com/
/// [`Config::connection_mode`]: config::Config::connection_mode
pub const SUPPORTED_VELOCITY_VERSION: (u16, u16, u16) = (3, 1, 2);

/// A discrete unit of time where 1 tick is the duration of a
/// single game update.
///
/// The duration of a game update on a Valence server depends on the current
/// configuration. In some contexts, "ticks" refer to the configured tick rate
/// while others refer to Minecraft's [standard TPS](STANDARD_TPS).
pub type Ticks = i64;

/// Minecraft's standard ticks per second (TPS).
pub const STANDARD_TPS: Ticks = 20;
