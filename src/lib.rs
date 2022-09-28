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
    unused_import_braces
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
#[doc(inline)]
pub use {uuid, valence_nbt as nbt, vek};

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
pub mod item;
pub mod itemstack;
pub mod player_list;
pub mod player_textures;
#[allow(dead_code)]
#[doc(hidden)]
pub mod protocol;
pub mod server;
mod slab;
mod slab_rc;
mod slab_versioned;
pub mod spatial_index;
pub mod text;
pub mod util;
pub mod world;

/// The Minecraft protocol version this library currently targets.
pub const PROTOCOL_VERSION: i32 = 760;
/// The name of the Minecraft version this library currently targets, e.g.
/// "1.8.2"
pub const VERSION_NAME: &str = "1.19.2";

/// The namespace for this library used internally for
/// [identifiers](crate::ident::Ident).
///
/// You should avoid using this namespace in your own identifiers.
pub const LIBRARY_NAMESPACE: &str = "valence";

/// A discrete unit of time where 1 tick is the duration of a
/// single game update.
///
/// The duration of a game update on a Valence server depends on the current
/// configuration. In some contexts, "ticks" refer to the configured tick rate
/// while others refer to Minecraft's [standard TPS](STANDARD_TPS).
pub type Ticks = i64;

/// Minecraft's standard ticks per second (TPS).
pub const STANDARD_TPS: Ticks = 20;
