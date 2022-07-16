//! A Rust framework for building Minecraft servers.
//!
//! Valence is a Rust library which provides the necessary abstractions over
//! Minecraft's protocol to build servers. Very few assumptions about the
//! desired server are made, which allows for greater flexibility in its design.
//!
//! At a high level, a Valence [`Server`] is a collection of [`Clients`],
//! [`Entities`], and [`Worlds`]. When a client connects to the server, they are
//! added to the server's [`Clients`]. After connecting, clients are assigned to
//! a [`World`] where they are able to interact with the entities and
//! [`Chunks`] that are a part of it.
//!
//! The Valence documentation assumes some familiarity with Minecraft and its
//! mechanics. See the [Minecraft Wiki] for general information and [wiki.vg]
//! for protocol documentation.
//!
//! [Minecraft Wiki]: https://minecraft.fandom.com/wiki/Minecraft_Wiki
//! [wiki.vg]: https://wiki.vg/Main_Page
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
//! [`Chunk`], [`Entity`], and [`Client`].
//!
//! **You must not call [`mem::swap`] on these references (or any other
//! function that would move their location in memory).** Doing so breaks
//! invariants within the library and the resulting behavior is safe but
//! unspecified. These types should be considered [pinned](std::pin).
//!
//! Preventing this illegal behavior using Rust's type system was considered too
//! cumbersome, so a note has been left here instead.
//!
//! [`mem::swap`]: std::mem::swap
//!
//! # Examples
//!
//! The following is a minimal server implementation. You should be able to
//! connect to the server at `localhost`.
//!
//! ```
//! use valence::config::Config;
//! use valence::server::{Server, ShutdownResult};
//!
//! pub fn main() -> ShutdownResult {
//!     valence::start_server(Game, ())
//! }
//!
//! struct Game;
//!
//! impl Config for Game {
//!     type ChunkData = ();
//!     type ClientData = ();
//!     type EntityData = ();
//!     type ServerData = ();
//!     type WorldData = ();
//!
//!     fn max_connections(&self) -> usize {
//!         256
//!     }
//!
//!     fn update(&self, server: &mut Server<Self>) {
//!         server.clients.retain(|_, client| {
//!             if client.created_tick() == server.shared.current_tick() {
//!                 println!("{} joined!", client.username());
//!             }
//!
//!             if client.is_disconnected() {
//!                 println!("{} left!", client.username());
//!                 false
//!             } else {
//!                 true
//!             }
//!         });
//! #        server.shared.shutdown::<_, std::convert::Infallible>(Ok(()));
//!     }
//! }
//! ```
//!
//! For more complete examples, see the [examples] in the source repository.
//!
//! [examples]: https://github.com/rj00a/valence/tree/main/examples
//!
//! # Feature Flags
//!
//! * `protocol`: Enables low-level access to the [`protocol`] module, which
//!   could be used to build your own proxy or client. This feature is
//!   considered experimental and is subject to change.
//!
//! [`Server`]: crate::server::Server
//! [`Clients`]: crate::client::Clients
//! [`Entities`]: crate::entity::Entities
//! [`Worlds`]: crate::world::Worlds
//! [`World`]: crate::world::World
//! [`Chunks`]: crate::chunk::Chunks
//! [`Chunk`]: crate::chunk::Chunk
//! [`Entity`]: crate::entity::Entity
//! [`Client`]: crate::client::Client

#![forbid(unsafe_code)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces
)]
#![allow(
    clippy::derive_partial_eq_without_eq,
    clippy::unusual_byte_groupings,
    clippy::comparison_chain
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
pub mod player_list;
pub mod player_textures;
#[allow(dead_code)]
mod protocol_inner;
pub mod server;
mod slotmap;
pub mod spatial_index;
pub mod text;
pub mod util;
pub mod world;

/// Provides low-level access to the Minecraft protocol.
#[cfg(feature = "protocol")]
pub mod protocol {
    pub use crate::protocol_inner::*;
}

/// Used on [`Config`](config::Config) to allow for async methods in traits.
///
/// For more information see the [async_trait] crate.
///
/// [async_trait]: https://docs.rs/async-trait/latest/async_trait/
pub use async_trait::async_trait;
#[doc(inline)]
pub use server::start_server;
#[doc(inline)]
pub use {nbt, uuid, vek};

/// The Minecraft protocol version this library currently targets.
pub const PROTOCOL_VERSION: i32 = 759;
/// The name of the Minecraft version this library currently targets, e.g.
/// "1.8.2"
pub const VERSION_NAME: &str = "1.19";

/// The namespace for this library used internally for
/// [identifiers](crate::ident::Ident).
///
/// You should avoid using this namespace in your own identifiers.
const LIBRARY_NAMESPACE: &str = "valence";

/// A discrete unit of time where 1 tick is the duration of a
/// single game update.
///
/// The duration of a game update depends on the current configuration, which
/// may or may not be the same as Minecraft's standard 20 ticks/second.
pub type Ticks = i64;
