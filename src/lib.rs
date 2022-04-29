#![forbid(unsafe_code)]
#![warn(
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    missing_docs
)]

mod aabb;
pub mod block;
mod block_pos;
mod byte_angle;
pub mod chunk;
pub mod client;
mod codec;
pub mod component;
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

pub use aabb::Aabb;
pub use block_pos::BlockPos;
pub use chunk::{Chunk, ChunkPos, ChunkStore};
pub use client::{Client, ClientStore};
pub use config::{BiomeId, DimensionId, ServerConfig};
pub use entity::{Entity, EntityId, EntityStore};
pub use identifier::Identifier;
pub use text::{Text, TextFormat};
pub use uuid::Uuid;
pub use world::{World, WorldId, WorldStore};
pub use {nalgebra_glm as glm, nbt, uuid};

pub use crate::server::{NewClientData, Server, SharedServer, ShutdownResult};

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

/// Types such as [`EntityId`], [`WorldId`], and [`ChunkId`] which can be used
/// as indices into an array.
///
/// Every ID is either valid or invalid. Valid IDs point to living values. For
/// instance, a valid [`EntityId`] points to a living entity on the server. When
/// that entity is deleted, the corresponding [`EntityId`] becomes invalid.
pub trait Id: Copy + Send + Sync + PartialEq + Eq {
    /// Returns the index of this ID.
    ///
    /// For all IDs `a` and `b`, `a == b` implies `a.idx() == b.idx()`. If
    /// both `a` and `b` are currently valid, then `a != b` implies `a.idx() !=
    /// b.idx()`.
    fn idx(self) -> usize;
}
