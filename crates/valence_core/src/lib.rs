//! A library for interacting with the Minecraft (Java Edition) network
//! protocol.
//!
//! The API is centered around the [`Encode`] and [`Decode`] traits. Clientbound
//! and serverbound packets are defined in the [`packet`] module. Packets are
//! encoded and decoded using the [`PacketEncoder`] and [`PacketDecoder`] types.
//!
//! [`PacketEncoder`]: encoder::PacketEncoder
//! [`PacketDecoder`]: decoder::PacketDecoder
//!
//! # Examples
//!
//! ```
//! use valence_protocol::decoder::PacketDecoder;
//! use valence_protocol::encoder::PacketEncoder;
//! use valence_protocol::packet::c2s::play::RenameItemC2s;
//! use valence_protocol::Packet;
//!
//! let mut enc = PacketEncoder::new();
//!
//! let outgoing = RenameItemC2s {
//!     item_name: "Hello!",
//! };
//!
//! enc.append_packet(&outgoing).unwrap();
//!
//! let mut dec = PacketDecoder::new();
//!
//! dec.queue_bytes(enc.take());
//!
//! let frame = dec.try_next_packet().unwrap().unwrap();
//!
//! let incoming = RenameItemC2s::decode_packet(&mut &frame[..]).unwrap();
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
#![allow(clippy::unusual_byte_groupings)]

// Allows us to use our own proc macros internally.
extern crate self as valence_protocol;

use std::fmt;
use std::io::Write;

pub use anyhow::{Error, Result};
pub use valence_protocol_macros::{ident, Decode, Encode, Packet};
pub use {bytes, uuid, valence_nbt as nbt};

/// The Minecraft protocol version this library currently targets.
pub const PROTOCOL_VERSION: i32 = 762;

/// The stringified name of the Minecraft version this library currently
/// targets.
pub const MINECRAFT_VERSION: &str = "1.19.4";

pub mod array;
pub mod block;
pub mod block_pos;
pub mod byte_angle;
pub mod decoder;
pub mod enchant;
pub mod encoder;
pub mod ident;
mod impls;
pub mod item;
pub mod packet;
pub mod raw;
pub mod sound;
pub mod text;
pub mod translation_key;
pub mod types;
pub mod var_int;
pub mod var_long;
pub mod aabb;

/// Used only by proc macros. Not public API.
#[doc(hidden)]
pub mod __private {
    pub use anyhow::{anyhow, bail, ensure, Context, Result};

    pub use crate::var_int::VarInt;
    pub use crate::{Decode, Encode, Packet};
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
/// [`VarInt`]: var_int::VarInt
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
    fn encode_slice(slice: &[Self], w: impl Write) -> Result<()>
    where
        Self: Sized,
    {
        let _ = (slice, w);
        unimplemented!("no implementation of `encode_slice`")
    }

    /// Hack to get around the lack of specialization. Not public API.
    #[doc(hidden)]
    const HAS_ENCODE_SLICE: bool = false;
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
/// [`VarInt`]: var_int::VarInt
pub trait Decode<'a>: Sized {
    /// Reads this object from the provided byte slice.
    ///
    /// Implementations of `Decode` are expected to shrink the slice from the
    /// front as bytes are read.
    fn decode(r: &mut &'a [u8]) -> Result<Self>;
}

/// Like [`Encode`] + [`Decode`], but implementations must read and write a
/// leading [`VarInt`] packet ID before any other data.
///
/// # Deriving
///
/// This trait can be implemented automatically by using the
/// [`Packet`][macro] derive macro. The trait is implemented by reading or
/// writing the packet ID provided in the `#[packet_id = ...]` helper attribute
/// followed by a call to [`Encode::encode`] or [`Decode::decode`]. The target
/// type must implement [`Encode`], [`Decode`], and [`fmt::Debug`].
///
/// ```
/// use valence_protocol::{Decode, Encode, Packet};
///
/// #[derive(Encode, Decode, Packet, Debug)]
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
/// [macro]: valence_protocol_macros::Packet
/// [`VarInt`]: var_int::VarInt
pub trait Packet<'a>: Sized + fmt::Debug {
    /// The packet returned by [`Self::packet_id`]. If the packet ID is not
    /// statically known, then a negative value is used instead.
    const PACKET_ID: i32 = -1;
    /// Returns the ID of this packet.
    fn packet_id(&self) -> i32;
    /// Returns the name of this packet, typically without whitespace or
    /// additional formatting.
    fn packet_name(&self) -> &str;
    /// Like [`Encode::encode`], but a leading [`VarInt`] packet ID must be
    /// written first.
    ///
    /// [`VarInt`]: var_int::VarInt
    fn encode_packet(&self, w: impl Write) -> Result<()>;
    /// Like [`Decode::decode`], but a leading [`VarInt`] packet ID must be read
    /// first.
    ///
    /// [`VarInt`]: var_int::VarInt
    fn decode_packet(r: &mut &'a [u8]) -> Result<Self>;
}

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use bytes::BytesMut;

    use super::*;
    use crate::decoder::{decode_packet, PacketDecoder};
    use crate::encoder::PacketEncoder;
    use crate::packet::c2s::play::HandSwingC2s;
    use crate::packet::C2sPlayPacket;

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet_id = 1]
    struct RegularStruct {
        foo: i32,
        bar: bool,
        baz: f64,
    }

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet_id = 2]
    struct UnitStruct;

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet_id = 3]
    struct EmptyStruct {}

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet_id = 4]
    struct TupleStruct(i32, bool, f64);

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet_id = 5]
    struct StructWithGenerics<'z, T = ()> {
        foo: &'z str,
        bar: T,
    }

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet_id = 6]
    struct TupleStructWithGenerics<'z, T = ()>(&'z str, i32, T);

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet_id = 7]
    enum RegularEnum {
        Empty,
        Tuple(i32, bool, f64),
        Fields { foo: i32, bar: bool, baz: f64 },
    }

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet_id = 8]
    enum EmptyEnum {}

    #[derive(Encode, Decode, Packet, Debug)]
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
    fn assert_has_impls<'a, T>()
    where
        T: Encode + Decode<'a> + Packet<'a>,
    {
        assert_has_impls::<RegularStruct>();
        assert_has_impls::<UnitStruct>();
        assert_has_impls::<EmptyStruct>();
        assert_has_impls::<TupleStruct>();
        assert_has_impls::<StructWithGenerics>();
        assert_has_impls::<TupleStructWithGenerics>();
        assert_has_impls::<RegularEnum>();
        assert_has_impls::<EmptyEnum>();
        assert_has_impls::<EnumWithGenericsAndTags>();
    }

    #[test]
    fn packet_name() {
        assert_eq!(UnitStruct.packet_name(), "UnitStruct");
        assert_eq!(RegularEnum::Empty.packet_name(), "RegularEnum");
        assert_eq!(
            StructWithGenerics {
                foo: "blah",
                bar: ()
            }
            .packet_name(),
            "StructWithGenerics"
        );
        assert_eq!(
            C2sPlayPacket::HandSwingC2s(HandSwingC2s {
                hand: Default::default()
            })
            .packet_name(),
            "HandSwingC2s"
        );
    }

    use crate::block_pos::BlockPos;
    use crate::ident::Ident;
    use crate::item::{ItemKind, ItemStack};
    use crate::text::{Text, TextFormat};
    use crate::types::Hand;
    use crate::var_int::VarInt;
    use crate::var_long::VarLong;

    #[cfg(feature = "encryption")]
    const CRYPT_KEY: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

    #[derive(PartialEq, Debug, Encode, Decode, Packet)]
    #[packet_id = 42]
    struct TestPacket<'a> {
        a: bool,
        b: u8,
        c: i32,
        d: f32,
        e: f64,
        f: BlockPos,
        g: Hand,
        h: Ident<Cow<'a, str>>,
        i: Option<ItemStack>,
        j: Text,
        k: VarInt,
        l: VarLong,
        m: &'a str,
        n: &'a [u8; 10],
        o: [u128; 3],
    }

    impl<'a> TestPacket<'a> {
        fn new(string: &'a str) -> Self {
            Self {
                a: true,
                b: 12,
                c: -999,
                d: 5.001,
                e: 1e10,
                f: BlockPos::new(1, 2, 3),
                g: Hand::Off,
                h: Ident::new("minecraft:whatever").unwrap(),
                i: Some(ItemStack::new(ItemKind::WoodenSword, 12, None)),
                j: "my ".into_text() + "fancy".italic() + " text",
                k: VarInt(123),
                l: VarLong(456),
                m: string,
                n: &[7; 10],
                o: [123456789; 3],
            }
        }
    }

    fn check_test_packet(dec: &mut PacketDecoder, string: &str) {
        let frame = dec.try_next_packet().unwrap().unwrap();

        let pkt = decode_packet::<TestPacket>(&frame).unwrap();

        assert_eq!(&pkt, &TestPacket::new(string));
    }

    #[test]
    fn packets_round_trip() {
        let mut buf = BytesMut::new();

        let mut enc = PacketEncoder::new();

        enc.append_packet(&TestPacket::new("first")).unwrap();
        #[cfg(feature = "compression")]
        enc.set_compression(Some(0));
        enc.append_packet(&TestPacket::new("second")).unwrap();
        buf.unsplit(enc.take());
        #[cfg(feature = "encryption")]
        enc.enable_encryption(&CRYPT_KEY);
        enc.append_packet(&TestPacket::new("third")).unwrap();
        enc.prepend_packet(&TestPacket::new("fourth")).unwrap();

        buf.unsplit(enc.take());

        let mut dec = PacketDecoder::new();

        dec.queue_bytes(buf);

        check_test_packet(&mut dec, "first");

        #[cfg(feature = "compression")]
        dec.set_compression(Some(0));

        check_test_packet(&mut dec, "second");

        #[cfg(feature = "encryption")]
        dec.enable_encryption(&CRYPT_KEY);

        check_test_packet(&mut dec, "fourth");
        check_test_packet(&mut dec, "third");
    }
}
