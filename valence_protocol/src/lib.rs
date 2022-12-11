//! A library for interacting with the Minecraft (Java Edition) network
//! protocol.
//!
//! The API is centered around the [`Encode`] and [`Decode`] traits. Clientbound
//! and serverbound packets are defined in the [`packets`] module. Packets are
//! encoded and decoded using the [`PacketEncoder`] and [`PacketDecoder`] types.
//!
//! # Examples
//!
//! ```
//! use valence_protocol::packets::c2s::play::RenameItem;
//! use valence_protocol::{PacketDecoder, PacketEncoder};
//!
//! let mut enc = PacketEncoder::new();
//!
//! let outgoing = RenameItem {
//!     item_name: "Hello!",
//! };
//!
//! enc.append_packet(&outgoing).unwrap();
//!
//! let mut dec = PacketDecoder::new();
//!
//! dec.queue_bytes(enc.take());
//!
//! let incoming = dec.try_next_packet::<RenameItem>().unwrap().unwrap();
//!
//! assert_eq!(outgoing.item_name, incoming.item_name);
//! ```
//!
//! # Stability
//!
//! The Minecraft protocol is not stable. Updates to Minecraft may change the
//! protocol in subtle or drastic ways. In response to this, `valence_protocol`
//! aims to support only the most recent version of the game (excluding
//! snapshots, pre-releases, etc). An update to Minecraft often requires a
//! breaking change to the library.
//!
//! `valence_protocol` is versioned in lockstep with `valence`. The currently
//! supported Minecraft version can be checked with the [`PROTOCOL_VERSION`] or
//! [`MINECRAFT_VERSION`] constants.
//!
//! # Feature Flags
//!
//! TODO

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
pub use block::{BlockFace, BlockKind, BlockState};
pub use block_pos::BlockPos;
pub use byte_angle::ByteAngle;
pub use cache::{Cached, EncodedBuf};
pub use codec::*;
pub use ident::Ident;
pub use inventory::InventoryKind;
pub use item::{ItemKind, ItemStack};
pub use raw_bytes::RawBytes;
pub use text::{Text, TextFormat};
pub use username::Username;
pub use uuid::Uuid;
pub use valence_derive::{Decode, Encode, Packet};
pub use var_int::VarInt;
pub use var_long::VarLong;
pub use {uuid, valence_nbt as nbt};

use crate::byte_counter::ByteCounter;

/// The Minecraft protocol version this library currently targets.
pub const PROTOCOL_VERSION: i32 = 760;

/// The stringified name of the Minecraft version this library currently
/// targets.
pub const MINECRAFT_VERSION: &str = "1.19.2";

pub mod block;
mod block_pos;
mod bounded;
mod byte_angle;
mod byte_counter;
mod cache;
mod codec;
pub mod enchant;
pub mod entity_meta;
pub mod ident;
mod impls;
mod inventory;
mod item;
pub mod packets;
mod raw_bytes;
pub mod text;
pub mod translation_key;
pub mod types;
pub mod username;
mod var_int;
mod var_long;

/// Used only by proc macros. Not public API.
#[doc(hidden)]
pub mod __private {
    pub use anyhow::{anyhow, bail, ensure, Context, Result};

    pub use crate::{Decode, DerivedPacketDecode, DerivedPacketEncode, Encode, VarInt};
}

/// The maximum number of bytes in a single Minecraft packet.
pub const MAX_PACKET_SIZE: i32 = 2097152;

/// The `Encode` trait allows objects to be written to the Minecraft protocol.
/// It is the inverse of [`Decode`].
///
/// # Deriving
///
/// This trait can be implemented automatically for structs and enums by using
/// the [`Encode`][macro] derive macro. All components of the type must
/// implement `Encode`. Components are encoded in the order they appear in the
/// type definition.
///
/// If a `#[packet_id = ...]` attribute is present, encoding the type begins by
/// writing the specified constant [`VarInt`] value before any of the
/// components.
///
/// For enums, the variant to encode is marked by a leading [`VarInt`]
/// discriminant (tag). The discriminant value can be changed using the `#[tag =
/// ...]` attribute on the variant in question. Discriminant values are assigned
/// to variants using rules similar to regular enum discriminants.
///
/// [`VarInt`]: var_int::VarInt
///
/// ```
/// use valence_protocol::Encode;
///
/// #[derive(Encode)]
/// #[packet_id = 42]
/// struct MyStruct<'a> {
///     first: i32,
///     second: &'a str,
///     third: [f64; 3],
/// }
///
/// #[derive(Encode)]
/// enum MyEnum {
///     First,  // tag = 0
///     Second, // tag = 1
///     #[tag = 25]
///     Third, // tag = 25
///     Fourth, // tag = 26
/// }
///
/// let value = MyStruct {
///     first: 10,
///     second: "hello",
///     third: [1.5, 3.14, 2.718],
/// };
///
/// let mut buf = vec![];
/// value.encode(&mut buf).unwrap();
///
/// println!("{buf:?}");
/// ```
///
/// [macro]: valence_derive::Encode
pub trait Encode {
    /// Writes this object to the provided writer.
    ///
    /// If this type also implements [`Decode`] then successful calls to this
    /// function returning `Ok(())` must always successfully [`decode`] using
    /// the data that was written to the writer. The exact number of bytes
    /// that were originally written must be consumed during the decoding.
    ///
    /// Additionally, this function must be pure. If no write error occurs,
    /// successive calls to `encode` must write the same bytes to the writer
    /// argument. This property can be broken by using internal mutability,
    /// global state, or other tricks.
    ///
    /// [`decode`]: Decode::decode
    fn encode(&self, w: impl Write) -> Result<()>;

    /// Returns the number of bytes that will be written when [`Self::encode`]
    /// is called.
    ///
    /// If [`Self::encode`] returns `Ok`, then the exact number of bytes
    /// reported by this function must be written to the writer argument.
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
/// `Decode` is parameterized by a lifetime. This allows the decoded value to
/// borrow data from the byte slice it was read from.
///
/// # Deriving
///
/// This trait can be implemented automatically for structs and enums by using
/// the [`Decode`][macro] derive macro. All components of the type must
/// implement `Decode`. Components are decoded in the order they appear in the
/// type definition.
///
/// If a `#[packet_id = ...]` attribute is present, encoding the type begins by
/// reading the specified constant [`VarInt`] value before any of the
/// components.
///
/// For enums, the variant to decode is determined by a leading [`VarInt`]
/// discriminant (tag). The discriminant value can be changed using the `#[tag =
/// ...]` attribute on the variant in question. Discriminant values are assigned
/// to variants using rules similar to regular enum discriminants.
///
/// ```
/// use valence_protocol::Decode;
///
/// #[derive(PartialEq, Debug, Decode)]
/// #[packet_id = 5]
/// struct MyStruct {
///     first: i32,
///     second: MyEnum,
/// }
///
/// #[derive(PartialEq, Debug, Decode)]
/// enum MyEnum {
///     First,  // tag = 0
///     Second, // tag = 1
///     #[tag = 25]
///     Third, // tag = 25
///     Fourth, // tag = 26
/// }
///
/// let mut r: &[u8] = &[5, 0, 0, 0, 0, 26];
///
/// let value = MyStruct::decode(&mut r).unwrap();
/// let expected = MyStruct {
///     first: 0,
///     second: MyEnum::Fourth,
/// };
///
/// assert_eq!(value, expected);
/// assert!(r.is_empty());
/// ```
///
/// [macro]: valence_derive::Decode
pub trait Decode<'a>: Sized {
    /// Reads this object from the provided byte slice.
    ///
    /// Implementations of `Decode` are expected to shrink the slice from the
    /// front as bytes are read.
    fn decode(r: &mut &'a [u8]) -> Result<Self>;
}

/// Marker for types that are encoded or decoded as complete packets.
///
/// A complete packet is data starting with a [`VarInt`] packet ID. [`Encode`]
/// and [`Decode`] implementations on `Self`, if present, are expected to handle
/// this leading `VarInt`.
pub trait Packet {
    /// The name of this packet.
    ///
    /// This is usually the name of the type representing the packet without any
    /// generic parameters or other decorations.
    fn packet_name(&self) -> &'static str;
}

/// Packets which obtained [`Encode`] implementations via the [`Encode`][macro]
/// derive macro with the `#[packet_id = ...]` attribute.
///
/// Along with [`DerivedPacketDecode`], this trait can be occasionally useful
/// for automating tasks such as defining large packet enums. Otherwise, this
/// trait should not be used and has thus been hidden from the documentation.
///
/// [macro]: valence_derive::Encode
#[doc(hidden)]
pub trait DerivedPacketEncode: Encode {
    /// The ID of this packet specified with `#[packet_id = ...]`.
    const ID: i32;
    /// The name of the type implementing this trait.
    const NAME: &'static str;

    /// Like [`Encode::encode`], but does not write a leading [`VarInt`] packet
    /// ID.
    fn encode_without_id(&self, w: impl Write) -> Result<()>;
    /// Like [`Encode::encoded_len`], but does not count a leading [`VarInt`]
    /// packet ID.
    fn encoded_len_without_id(&self) -> usize;
}

/// Packets which obtained [`Decode`] implementations via the [`Decode`][macro]
/// derive macro with the `#[packet_id = ...]` attribute.
///
/// Along with [`DerivedPacketEncode`], this trait can be occasionally useful
/// for automating tasks such as defining large packet enums. Otherwise, this
/// trait should not be used and has thus been hidden from the documentation.
///
/// [macro]: valence_derive::Decode
#[doc(hidden)]
pub trait DerivedPacketDecode<'a>: Decode<'a> {
    /// The ID of this packet specified with `#[packet_id = ...]`.
    const ID: i32;
    /// The name of the type implementing this trait.
    const NAME: &'static str;

    /// Like [`Decode::decode`], but does not decode a leading [`VarInt`] packet
    /// ID.
    fn decode_without_id(r: &mut &'a [u8]) -> Result<Self>;
}

#[allow(dead_code)]
#[cfg(test)]
mod derive_tests {
    use super::*;

    #[derive(Encode, Decode, Packet)]
    #[packet_id = 1]
    struct RegularStruct {
        foo: i32,
        bar: bool,
        baz: f64,
    }

    #[derive(Encode, Decode, Packet)]
    #[packet_id = 2]
    struct UnitStruct;

    #[derive(Encode, Decode, Packet)]
    #[packet_id = 3]
    struct EmptyStruct {}

    #[derive(Encode, Decode, Packet)]
    #[packet_id = 4]
    struct TupleStruct(i32, bool, f64);

    #[derive(Encode, Decode, Packet)]
    #[packet_id = 5]
    struct StructWithGenerics<'z, T = ()> {
        foo: &'z str,
        bar: T,
    }

    #[derive(Encode, Decode, Packet)]
    #[packet_id = 6]
    struct TupleStructWithGenerics<'z, T = ()>(&'z str, i32, T);

    #[derive(Encode, Decode, Packet)]
    #[packet_id = 7]
    enum RegularEnum {
        Empty,
        Tuple(i32, bool, f64),
        Fields { foo: i32, bar: bool, baz: f64 },
    }

    #[derive(Encode, Decode, Packet)]
    #[packet_id = 8]
    enum EmptyEnum {}

    #[derive(Encode, Decode, Packet)]
    #[packet_id = 0xbeef]
    enum EnumWithGenericsAndTags<'z, T = ()> {
        #[tag = 5]
        First {
            foo: &'z str,
        },
        Second(&'z str),
        #[tag = 0xff]
        Third,
        #[tag = 0]
        Fourth(T),
    }

    #[allow(unconditional_recursion)]
    fn has_impls<'a, T>()
    where
        T: Encode + Decode<'a> + DerivedPacketEncode + DerivedPacketDecode<'a> + Packet,
    {
        has_impls::<RegularStruct>();
        has_impls::<UnitStruct>();
        has_impls::<EmptyStruct>();
        has_impls::<TupleStruct>();
        has_impls::<StructWithGenerics>();
        has_impls::<TupleStructWithGenerics>();
        has_impls::<RegularEnum>();
        has_impls::<EmptyEnum>();
        has_impls::<EnumWithGenericsAndTags>();
    }
}
