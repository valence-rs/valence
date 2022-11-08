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

use anyhow::ensure;
pub use anyhow::{Error, Result};
pub use valence_derive::{Decode, Encode, Packet};
pub use valence_nbt as nbt;

use crate::byte_counter::ByteCounter;

pub mod block;
pub mod block_pos;
pub mod bounded;
pub mod byte_angle;
mod byte_counter;
pub mod codec;
pub mod enchant;
pub mod encoded_buf;
pub mod ident;
mod impls;
pub mod item;
pub mod packets;
pub mod raw_bytes;
pub mod text;
pub mod username;
pub mod var_int;
pub mod var_long;

/// Used only by proc macros. Not public API.
#[doc(hidden)]
pub mod __private {
    pub use anyhow::{anyhow, bail, ensure, Context, Result};

    pub use crate::var_int::VarInt;
    pub use crate::{Decode, DerivedPacketDecode, DerivedPacketEncode, Encode};
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

/// Packets which obtained [`Encode`] implementations via the [`Encode`][macro]
/// derive macro with the `#[packet_id = ...]` attribute.
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
    /// The name of the `Self` type.
    const NAME: &'static str;

    fn encode_without_id(&self, w: impl Write) -> Result<()>;
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
pub trait DerivedPacketDecode<'a>: Packet + Decode<'a> {
    /// The ID of this packet specified with `#[packet_id = ...]`.
    const ID: i32;
    /// The name of the `Self` type.
    const NAME: &'static str;

    fn decode_without_id(r: &mut &'a [u8]) -> Result<Self>;
}

pub fn test_encode_decode<T>(t: &T) -> Result<()>
where
    T: Encode + for<'a> Decode<'a>,
{
    let len = t.encoded_len();
    let mut buf = Vec::with_capacity(len);

    if let Err(_) = t.encode(&mut buf) {
        ensure!(
            buf.len() <= len,
            "number of written bytes is larger than expected"
        );
        return Ok(());
    }

    let mut r = buf.as_slice();

    match T::decode(&mut r) {
        Ok(_) => {
            ensure!(
                r.is_empty(),
                "not all bytes were read after successful decode ({} bytes remain)",
                r.len()
            );
            Ok(())
        }
        Err(e) => Err(e.context("failed to decode after successfully encoding")),
    }
}

#[allow(dead_code)]
mod derive_tests {
    use crate::{Decode, Encode, Packet};

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
    struct StructWithLifetime<'z> {
        foo: &'z str,
    }

    #[derive(Encode, Decode, Packet)]
    #[packet_id = 6]
    struct TupleStructWithLifetime<'z>(&'z str, i32);

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
    enum EnumWithLifetime<'z> {
        First { foo: &'z str },
        Second(&'z str),
        Third,
    }
}
