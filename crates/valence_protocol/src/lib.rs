#![doc = include_str!("../README.md")]
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::bare_urls,
    rustdoc::invalid_html_tags
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    unreachable_pub,
    clippy::dbg_macro
)]

/// Used only by macros. Not public API.
#[doc(hidden)]
pub mod __private {
    pub use anyhow::{anyhow, bail, ensure, Context, Result};

    pub use crate::var_int::VarInt;
    pub use crate::{Decode, Encode, Packet};
}

// This allows us to use our own proc macros internally.
extern crate self as valence_protocol;

mod array;
mod biome_pos;
mod bit_set;
pub mod block_pos;
mod bounded;
mod byte_angle;
pub mod chunk_pos;
pub mod chunk_section_pos;
pub mod decode;
mod difficulty;
mod direction;
pub mod encode;
pub mod game_mode;
mod global_pos;
mod hand;
mod impls;
pub mod item;
pub mod packets;
pub mod profile;
mod raw;
pub mod sound;
pub mod var_int;
mod var_long;
mod velocity;

use std::io::Write;

use anyhow::Context;
pub use array::FixedArray;
pub use biome_pos::BiomePos;
pub use bit_set::FixedBitSet;
pub use block::{BlockKind, BlockState};
pub use block_pos::BlockPos;
pub use bounded::Bounded;
pub use byte_angle::ByteAngle;
pub use chunk_pos::ChunkPos;
pub use chunk_section_pos::ChunkSectionPos;
pub use decode::PacketDecoder;
use derive_more::{From, Into};
pub use difficulty::Difficulty;
pub use direction::Direction;
pub use encode::{PacketEncoder, WritePacket};
pub use game_mode::GameMode;
pub use global_pos::GlobalPos;
pub use hand::Hand;
pub use ident::ident;
pub use item::{ItemKind, ItemStack};
pub use packets::play::particle_s2c::Particle;
pub use raw::RawBytes;
pub use sound::Sound;
pub use text::Text;
pub use valence_generated::{block, packet_id};
pub use valence_ident::Ident;
pub use valence_protocol_macros::{Decode, Encode, Packet};
pub use var_int::VarInt;
pub use var_long::VarLong;
pub use velocity::Velocity;
pub use {
    anyhow, bytes, uuid, valence_ident as ident, valence_math as math, valence_nbt as nbt,
    valence_text as text,
};

/// The maximum number of bytes in a single Minecraft packet.
pub const MAX_PACKET_SIZE: i32 = 2097152;

/// The Minecraft protocol version this library currently targets.
pub const PROTOCOL_VERSION: i32 = 763;

/// The stringified name of the Minecraft version this library currently
/// targets.
pub const MINECRAFT_VERSION: &str = "1.20.1";

/// How large a packet should be before it is compressed by the packet encoder.
///
/// If the inner value is >= 0, then packets with encoded lengths >= to this
/// value will be compressed. If the value is negative, then compression is
/// disabled and no packets are compressed.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, From, Into)]
pub struct CompressionThreshold(pub i32);

impl CompressionThreshold {
    /// No compression.
    pub const DEFAULT: Self = Self(-1);
}

/// No compression.
impl Default for CompressionThreshold {
    fn default() -> Self {
        Self::DEFAULT
    }
}

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
/// discriminant (tag). The discriminant value can be changed using the
/// `#[packet(tag = ...)]` attribute on the variant in question. Discriminant
/// values are assigned to variants using rules similar to regular enum
/// discriminants.
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
///     #[packet(tag = 25)]
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
    fn encode(&self, w: impl Write) -> anyhow::Result<()>;

    /// Like [`Encode::encode`], except that a whole slice of values is encoded.
    ///
    /// This method must be semantically equivalent to encoding every element of
    /// the slice in sequence with no leading length prefix (which is exactly
    /// what the default implementation does), but a more efficient
    /// implementation may be used.
    ///
    /// This method is important for some types like `u8` where the entire slice
    /// can be encoded in a single call to [`write_all`]. Because impl
    /// specialization is unavailable in stable Rust at the time of writing,
    /// we must make the slice specialization part of this trait.
    ///
    /// [`write_all`]: Write::write_all
    fn encode_slice(slice: &[Self], mut w: impl Write) -> anyhow::Result<()>
    where
        Self: Sized,
    {
        for value in slice {
            value.encode(&mut w)?;
        }

        Ok(())
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
/// For enums, the variant to decode is determined by a leading [`VarInt`]
/// discriminant (tag). The discriminant value can be changed using the
/// `#[packet(tag = ...)]` attribute on the variant in question. Discriminant
/// values are assigned to variants using rules similar to regular enum
/// discriminants.
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
///     #[packet(tag = 25)]
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
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self>;
}

/// Types considered to be Minecraft packets.
///
/// In serialized form, a packet begins with a [`VarInt`] packet ID followed by
/// the body of the packet. If present, the implementations of [`Encode`] and
/// [`Decode`] on `Self` are expected to only encode/decode the _body_ of this
/// packet without the leading ID.
pub trait Packet: std::fmt::Debug {
    /// The leading VarInt ID of this packet.
    const ID: i32;
    /// The name of this packet for debugging purposes.
    const NAME: &'static str;
    /// The side this packet is intended for.
    const SIDE: PacketSide;
    /// The state in which this packet is used.
    const STATE: PacketState;

    /// Encodes this packet's VarInt ID first, followed by the packet's body.
    fn encode_with_id(&self, mut w: impl Write) -> anyhow::Result<()>
    where
        Self: Encode,
    {
        VarInt(Self::ID)
            .encode(&mut w)
            .context("failed to encode packet ID")?;

        self.encode(w)
    }
}

/// The side a packet is intended for.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PacketSide {
    /// Server -> Client
    Clientbound,
    /// Client -> Server
    Serverbound,
}

/// The statein  which a packet is used.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PacketState {
    Handshaking,
    Status,
    Login,
    Play,
}

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use bytes::BytesMut;

    use super::*;
    use crate::block_pos::BlockPos;
    use crate::decode::PacketDecoder;
    use crate::encode::PacketEncoder;
    use crate::hand::Hand;
    use crate::item::{ItemKind, ItemStack};
    use crate::text::{IntoText, Text};
    use crate::var_int::VarInt;
    use crate::var_long::VarLong;
    use crate::Ident;

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet(id = 1, side = PacketSide::Clientbound)]
    struct RegularStruct {
        foo: i32,
        bar: bool,
        baz: f64,
    }

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet(id = 2, side = PacketSide::Clientbound)]
    struct UnitStruct;

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet(id = 3, side = PacketSide::Clientbound)]
    struct EmptyStruct {}

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet(id = 4, side = PacketSide::Clientbound)]
    struct TupleStruct(i32, bool, f64);

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet(id = 5, side = PacketSide::Clientbound)]
    struct StructWithGenerics<'z, T = ()> {
        foo: &'z str,
        bar: T,
    }

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet(id = 6, side = PacketSide::Clientbound)]
    struct TupleStructWithGenerics<'z, T = ()>(&'z str, i32, T);

    #[allow(unconditional_recursion, clippy::extra_unused_type_parameters)]
    fn assert_has_impls<'a, T>()
    where
        T: Encode + Decode<'a> + Packet,
    {
        assert_has_impls::<RegularStruct>();
        assert_has_impls::<UnitStruct>();
        assert_has_impls::<EmptyStruct>();
        assert_has_impls::<TupleStruct>();
        assert_has_impls::<StructWithGenerics>();
        assert_has_impls::<TupleStructWithGenerics>();
    }

    #[test]
    fn packet_name() {
        assert_eq!(RegularStruct::NAME, "RegularStruct");
        assert_eq!(UnitStruct::NAME, "UnitStruct");
        assert_eq!(StructWithGenerics::<()>::NAME, "StructWithGenerics");
    }

    #[cfg(feature = "encryption")]
    const CRYPT_KEY: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

    #[derive(PartialEq, Debug, Encode, Decode, Packet)]
    #[packet(id = 42, side = PacketSide::Clientbound)]
    struct TestPacket<'a> {
        a: bool,
        b: u8,
        c: i32,
        d: f32,
        e: f64,
        f: BlockPos,
        g: Hand,
        h: Ident<Cow<'a, str>>,
        i: ItemStack,
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
                i: ItemStack::new(ItemKind::WoodenSword, 12, None),
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

        let pkt = frame.decode::<TestPacket>().unwrap();

        assert_eq!(&pkt, &TestPacket::new(string));
    }

    #[test]
    fn packets_round_trip() {
        let mut buf = BytesMut::new();

        let mut enc = PacketEncoder::new();

        enc.append_packet(&TestPacket::new("first")).unwrap();
        #[cfg(feature = "compression")]
        enc.set_compression(0.into());
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
        dec.set_compression(0.into());

        check_test_packet(&mut dec, "second");

        #[cfg(feature = "encryption")]
        dec.enable_encryption(&CRYPT_KEY);

        check_test_packet(&mut dec, "fourth");
        check_test_packet(&mut dec, "third");
    }
}
