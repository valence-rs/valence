#![forbid(unsafe_code)]

mod aabb;
mod block_pos;
mod byte_angle;
mod chunk;
mod chunk_store;
mod client;
mod codec;
pub mod component;
pub mod config;
pub mod entity;
pub mod identifier;
mod packets;
mod protocol;
mod server;
pub mod text;
pub mod util;
mod var_int;
mod var_long;
mod world;
pub mod block;

pub use aabb::Aabb;
pub use chunk::{Chunk, ChunkPos};
pub use client::Client;
pub use config::{BiomeId, DimensionId, ServerConfig};
pub use entity::{EntityId, EntityStore};
pub use identifier::Identifier;
pub use text::{Text, TextFormat};
pub use uuid::Uuid;
pub use world::{World, WorldId};
pub use {nalgebra_glm as glm, nbt};

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
