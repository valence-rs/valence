#![forbid(unsafe_code)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    // missing_docs
)]

pub mod biome;
pub mod block;
mod block_pos;
mod bvh;
pub mod chunk;
mod chunk_pos;
pub mod client;
pub mod config;
pub mod dimension;
pub mod entity;
pub mod ident;
mod player_list;
pub mod player_textures;
#[cfg(not(feature = "protocol"))]
#[allow(unused)]
mod protocol;
#[cfg(feature = "protocol")]
pub mod protocol;
pub mod server;
mod slotmap;
mod spatial_index;
pub mod text;
pub mod util;
pub mod world;

pub use async_trait::async_trait;
pub use server::start_server;
pub use spatial_index::SpatialIndex;
pub use {nbt, uuid, vek};

/// The Minecraft protocol version that this library targets.
pub const PROTOCOL_VERSION: i32 = 759;
/// The name of the Minecraft version that this library targets.
pub const VERSION_NAME: &str = "1.19";

/// The namespace for this library used internally for namespaced identifiers.
const LIBRARY_NAMESPACE: &str = "valence";

/// A discrete unit of time where 1 tick is the duration of a
/// single game update.
///
/// The duration of a game update depends on the current configuration, which
/// may or may not be the same as Minecraft's standard 20 ticks/second.
pub type Ticks = i64;
