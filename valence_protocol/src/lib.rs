#![forbid(unsafe_code)]
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

// Allows us to use our own proc macros internally.
extern crate self as valence_protocol;

use std::io::Write;

pub use anyhow::{Error, Result};
pub use valence_derive::{Decode, Encode};
pub use valence_nbt as nbt;

use crate::byte_counter::ByteCounter;

pub mod block;
pub mod bounded;
pub mod byte_angle;
mod byte_counter;
pub mod codec;
pub mod enchant;
pub mod ident;
mod impls;
pub mod item;
pub mod raw_bytes;
pub mod text;
pub mod username;
pub mod var_int;
pub mod var_long;
pub mod block_pos;

/// Used only by proc macros. Not public API.
#[doc(hidden)]
pub mod __private {
    pub use anyhow::{anyhow, bail, Context, Result};

    pub use crate::var_int::VarInt;
    pub use crate::{Decode, Encode};
}

/// The maximum number of bytes in a single Minecraft packet.
pub const MAX_PACKET_SIZE: i32 = 2097152;

/// The `Encode` trait allows objects to be written to the Minecraft protocol.
/// It is the inverse of [`Decode`].
///
/// This trait can be implemented automatically by using the [`Encode`][macro]
/// derive macro.
///
/// [macro]: valence_derive::Encode
pub trait Encode {
    /// Writes this object to the provided writer.
    ///
    /// TODO: mention that on success, Decode must always succeed.
    ///
    /// This method must be pure. In other words, successive calls to `encode`
    /// must write the same bytes to the writer argument assuming no write
    /// error occurs. This property can be broken by using internal
    /// mutability, global state, or other tricks.
    fn encode(&self, w: impl Write) -> Result<()>;

    /// Returns the number of bytes that will be written when [`Self::encode`]
    /// is called.
    ///
    /// If [`Self::encode`] results in `Ok`, the exact number of bytes reported
    /// by this function must be written to the writer argument.
    ///
    /// If the result is `Err`, then the number of written bytes must be less
    /// than or equal to the count returned by this function.
    ///
    /// # Default Implementation
    ///
    /// Calls [`Self::encode`] to count the number of written bytes. This is
    /// always correct, but is not always the most efficient approach.
    fn encoded_len(&self) -> usize {
        let mut counter = ByteCounter::new();
        let _ = self.encode(&mut counter);
        counter.0
    }
}

/// The `Decode` trait allows objects to be read from the Minecraft protocol. It
/// is the inverse of [`Encode`].
///
/// This trait can be implemented automatically by using the [`Decode`][macro]
/// derive macro.
///
/// [macro]: valence_derive::Decode
pub trait Decode<'a>: Sized {
    fn decode(r: &mut &'a [u8]) -> Result<Self>;
}

/// Marker for types that are encoded or decoded as complete packets.
///
/// A complete packet is data starting with a [`VarInt`] packet ID. [`Encode`]
/// and [`Decode`] implementations on `Self`, if present, are expected to handle
/// this leading `VarInt`.
///
/// [`VarInt`]: var_int::VarInt
pub trait Packet {
    /// The name of this packet.
    ///
    /// This is usually the name of the type representing the packet without any
    /// generic parameters or other decorations.
    fn packet_name(&self) -> &'static str;
}

/// Packets which obtained [`Encode`] and [`Packet`] implementations via the
/// [`Encode`][macro] derive macro.
///
/// Along with [`DerivedPacketDecode`], this trait can be occasionally useful
/// for automating tasks such as defining large packet enums. Otherwise, this
/// trait should not be used and has thus been hidden from the documentation.
///
/// [macro]: valence_derive::Encode
#[doc(hidden)]
pub trait DerivedPacketEncode: Packet + Encode {
    /// The ID of this packet specified with `#[packet_id = ...]`.
    const ID: i32;

    fn encode_without_id(&self, w: impl Write) -> Result<()>;
    fn encoded_len_without_id(&self) -> usize;
}

/// Packets which obtained [`Decode`] and [`Packet`] implementations via the
/// [`Decode`][macro] derive macro.
///
/// Along with [`DerivedPacketEncode`], this trait can be occasionally useful
/// for automating tasks such as defining large packet enums. Otherwise, this
/// trait should not be used and has thus been hidden from the documentation.
///
/// [macro]: valence_derive::Decode
#[doc(hidden)]
pub trait DerivedPacketDecode<'a>: Packet + Decode<'a> {
    /// The ID of this packet specified with `#[packet_id = ...]`.
    const ID: i32;

    fn decode_without_id(r: &mut &'a [u8]) -> Result<Self>;
}
