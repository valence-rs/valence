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
#![allow(clippy::unusual_byte_groupings)]

/// Used only by macros. Not public API.
#[doc(hidden)]
pub mod __private {
    pub use anyhow::{anyhow, bail, ensure, Context, Result};

    pub use crate::packet::var_int::VarInt;
    pub use crate::packet::{Decode, Encode, Packet};
}

// Needed to make proc macros work.
extern crate self as valence_core;

/// The Minecraft protocol version this library currently targets.
pub const PROTOCOL_VERSION: i32 = 762;

/// The stringified name of the Minecraft version this library currently
/// targets.
pub const MINECRAFT_VERSION: &str = "1.19.4";

pub mod aabb;
pub mod block_pos;
pub mod chunk_pos;
pub mod despawn;
pub mod difficulty;
pub mod direction;
pub mod game_mode;
pub mod hand;
pub mod ident;
pub mod item;
pub mod packet;
pub mod player_textures;
pub mod property;
pub mod scratch;
pub mod sound;
pub mod text;
pub mod translation_key;
pub mod uuid;
