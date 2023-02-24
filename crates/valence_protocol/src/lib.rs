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
use std::{fmt, io};

pub use anyhow::{Error, Result};
pub use array::LengthPrefixedArray;
pub use block::{BlockFace, BlockKind, BlockState};
pub use block_pos::BlockPos;
pub use byte_angle::ByteAngle;
pub use codec::*;
pub use ident::Ident;
pub use item::{ItemKind, ItemStack};
pub use raw_bytes::RawBytes;
pub use sound::Sound;
pub use text::{Text, TextFormat};
pub use username::Username;
pub use uuid::Uuid;
pub use valence_protocol_macros::{Decode, DecodePacket, Encode, EncodePacket};
pub use var_int::VarInt;
pub use var_long::VarLong;
pub use {uuid, valence_nbt as nbt};

/// The Minecraft protocol version this library currently targets.
pub const PROTOCOL_VERSION: i32 = 761;

/// The stringified name of the Minecraft version this library currently
/// targets.
pub const MINECRAFT_VERSION: &str = "1.19.3";

mod array;
pub mod block;
mod block_pos;
mod bounded;
mod byte_angle;
mod codec;
pub mod enchant;
pub mod entity_meta;
pub mod ident;
mod impls;
mod item;
pub mod packets;
mod raw_bytes;
pub mod sound;
pub mod text;
pub mod translation_key;
pub mod types;
pub mod username;
pub mod var_int;
mod var_long;

/// Used only by proc macros. Not public API.
#[doc(hidden)]
pub mod __private {
    pub use anyhow::{anyhow, bail, ensure, Context, Result};

    pub use crate::{Decode, DecodePacket, Encode, EncodePacket, VarInt};
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
/// For enums, the variant to encode is marked by a leading [`VarInt`]
/// discriminant (tag). The discriminant value can be changed using the `#[tag =
/// ...]` attribute on the variant in question. Discriminant values are assigned
/// to variants using rules similar to regular enum discriminants.
///
/// ```
/// use valence_protocol::Encode;
///
/// #[derive(Encode)]
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
/// [macro]: valence_protocol_macros::Encode
pub trait Encode {
    /// Writes this object to the provided writer.
    ///
    /// If this type also implements [`Decode`] then successful calls to this
    /// function returning `Ok(())` must always successfully [`decode`] using
    /// the data that was written to the writer. The exact number of bytes
    /// that were originally written must be consumed during the decoding.
    ///
    /// [`decode`]: Decode::decode
    fn encode(&self, w: impl Write) -> Result<()>;

    /// Hack to get around the lack of specialization. Not public API.
    #[doc(hidden)]
    fn write_slice(slice: &[Self], w: impl Write) -> io::Result<()>
    where
        Self: Sized,
    {
        let _ = (slice, w);
        unimplemented!("for internal use in valence_protocol only")
    }

    /// Hack to get around the lack of specialization. Not public API.
    #[doc(hidden)]
    const HAS_WRITE_SLICE: bool = false;
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
/// For enums, the variant to decode is determined by a leading [`VarInt`]
/// discriminant (tag). The discriminant value can be changed using the `#[tag =
/// ...]` attribute on the variant in question. Discriminant values are assigned
/// to variants using rules similar to regular enum discriminants.
///
/// ```
/// use valence_protocol::Decode;
///
/// #[derive(PartialEq, Debug, Decode)]
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
/// let mut r: &[u8] = &[0, 0, 0, 0, 26];
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
/// [macro]: valence_protocol_macros::Decode
pub trait Decode<'a>: Sized {
    /// Reads this object from the provided byte slice.
    ///
    /// Implementations of `Decode` are expected to shrink the slice from the
    /// front as bytes are read.
    fn decode(r: &mut &'a [u8]) -> Result<Self>;
}

/// Like [`Encode`], but implementations must write a leading [`VarInt`] packet
/// ID before any other data.
///
/// # Deriving
///
/// This trait can be implemented automatically by using the
/// [`EncodePacket`][macro] derive macro. The trait is implemented by writing
/// the packet ID provided in the `#[packet_id = ...]` helper attribute followed
/// by a call to [`Encode::encode`].
///
/// ```
/// use valence_protocol::{Encode, EncodePacket};
///
/// #[derive(Encode, EncodePacket, Debug)]
/// #[packet_id = 42]
/// struct MyStruct {
///     first: i32,
/// }
///
/// let value = MyStruct { first: 123 };
/// let mut buf = vec![];
///
/// value.encode_packet(&mut buf).unwrap();
/// println!("{buf:?}");
/// ```
///
/// [macro]: valence_protocol_macros::DecodePacket
pub trait EncodePacket: fmt::Debug {
    /// The packet ID that is written when [`Self::encode_packet`] is called. A
    /// negative value indicates that the packet ID is not statically known.
    const PACKET_ID: i32 = -1;

    /// Like [`Encode::encode`], but a leading [`VarInt`] packet ID must be
    /// written first.
    fn encode_packet(&self, w: impl Write) -> Result<()>;
}

/// Like [`Decode`], but implementations must read a leading [`VarInt`] packet
/// ID before any other data.
///
/// # Deriving
///
/// This trait can be implemented automatically by using the
/// [`DecodePacket`][macro] derive macro. The trait is implemented by reading
/// the packet ID provided in the `#[packet_id = ...]` helper attribute followed
/// by a call to [`Decode::decode`].
///
/// ```
/// use valence_protocol::{Decode, DecodePacket};
///
/// #[derive(Decode, DecodePacket, Debug)]
/// #[packet_id = 42]
/// struct MyStruct {
///     first: i32,
/// }
///
/// let buf = [42, 0, 0, 0, 0];
/// let mut r = buf.as_slice();
///
/// let value = MyStruct::decode_packet(&mut r).unwrap();
///
/// assert_eq!(value.first, 0);
/// assert!(r.is_empty());
/// ```
///
/// [macro]: valence_protocol::DecodePacket
pub trait DecodePacket<'a>: Sized + fmt::Debug {
    /// The packet ID that is read when [`Self::decode_packet`] is called. A
    /// negative value indicates that the packet ID is not statically known.
    const PACKET_ID: i32 = -1;

    /// Like [`Decode::decode`], but a leading [`VarInt`] packet ID must be read
    /// first.
    fn decode_packet(r: &mut &'a [u8]) -> Result<Self>;
}

#[allow(dead_code)]
#[cfg(test)]
mod derive_tests {
    use super::*;

    #[derive(Encode, EncodePacket, Decode, DecodePacket, Debug)]
    #[packet_id = 1]
    struct RegularStruct {
        foo: i32,
        bar: bool,
        baz: f64,
    }

    #[derive(Encode, EncodePacket, Decode, DecodePacket, Debug)]
    #[packet_id = 2]
    struct UnitStruct;

    #[derive(Encode, EncodePacket, Decode, DecodePacket, Debug)]
    #[packet_id = 3]
    struct EmptyStruct {}

    #[derive(Encode, EncodePacket, Decode, DecodePacket, Debug)]
    #[packet_id = 4]
    struct TupleStruct(i32, bool, f64);

    #[derive(Encode, EncodePacket, Decode, DecodePacket, Debug)]
    #[packet_id = 5]
    struct StructWithGenerics<'z, T: std::fmt::Debug = ()> {
        foo: &'z str,
        bar: T,
    }

    #[derive(Encode, EncodePacket, Decode, DecodePacket, Debug)]
    #[packet_id = 6]
    struct TupleStructWithGenerics<'z, T: std::fmt::Debug = ()>(&'z str, i32, T);

    #[derive(Encode, EncodePacket, Decode, DecodePacket, Debug)]
    #[packet_id = 7]
    enum RegularEnum {
        Empty,
        Tuple(i32, bool, f64),
        Fields { foo: i32, bar: bool, baz: f64 },
    }

    #[derive(Encode, EncodePacket, Decode, DecodePacket, Debug)]
    #[packet_id = 8]
    enum EmptyEnum {}

    #[derive(Encode, EncodePacket, Decode, DecodePacket, Debug)]
    #[packet_id = 0xbeef]
    enum EnumWithGenericsAndTags<'z, T: std::fmt::Debug = ()> {
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
        T: Encode + EncodePacket + Decode<'a> + DecodePacket<'a>,
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
