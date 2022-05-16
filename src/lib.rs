#![forbid(unsafe_code)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    // missing_docs
)]

pub mod block;
mod block_pos;
mod byte_angle;
pub mod chunk;
pub mod client;
mod codec;
pub mod config;
pub mod entity;
pub mod identifier;
mod packets;
mod protocol;
pub mod server;
mod slotmap;
pub mod text;
pub mod util;
mod var_int;
mod var_long;
pub mod world;

pub use async_trait::async_trait;
pub use block_pos::BlockPos;
pub use chunk::{Chunk, ChunkPos, Chunks, ChunksMut};
pub use client::{Client, ClientMut, Clients, ClientsMut};
pub use config::{Biome, BiomeId, Config, Dimension, DimensionId};
pub use entity::{Entities, EntitiesMut, Entity, EntityId};
pub use identifier::Identifier;
pub use server::{start_server, NewClientData, Server, ShutdownResult};
pub use text::{Text, TextFormat};
pub use uuid::Uuid;
pub use world::{WorldId, WorldMut, WorldRef, Worlds, WorldsMut};
pub use {nbt, uuid, vek};

/// The Minecraft protocol version that this library targets.
pub const PROTOCOL_VERSION: i32 = 758;
/// The name of the Minecraft version that this library targets.
pub const VERSION_NAME: &str = "1.18.2";

/// The namespace for this library used internally for namespaced identifiers.
const LIBRARY_NAMESPACE: &str = "valence";

/// A discrete unit of time where 1 tick is the duration of a
/// single game update.
///
/// The duration of a game update depends on the current configuration, which
/// may or may not be the same as Minecraft's standard 20 ticks/second.
pub type Ticks = i64;
