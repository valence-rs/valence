//! Types and functions used in Minecraft's packets. Structs for each packet are
//! defined here too.
//!
//! Client-to-server packets live in [`c2s`] while server-to-client packets are
//! in [`s2c`].

pub mod array;
pub mod byte_angle;
pub mod decode;
pub mod encode;
pub mod global_pos;
pub mod impls;
pub mod packet;
pub mod raw;
pub mod var_int;
pub mod var_long;

use std::io::Write;

use anyhow::Context;
pub use valence_core_macros::{Decode, Encode, Packet};
use var_int::VarInt;

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
/// use valence_core::protocol::Encode;
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
/// [macro]: valence_core_macros::Encode
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
    /// This optimization is very important for some types like `u8` where
    /// [`write_all`] is used. Because impl specialization is unavailable in
    /// stable Rust, we must make the slice specialization part of this trait.
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
/// discriminant (tag). The discriminant value can be changed using the `#[tag =
/// ...]` attribute on the variant in question. Discriminant values are assigned
/// to variants using rules similar to regular enum discriminants.
///
/// ```
/// use valence_core::protocol::Decode;
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
/// [macro]: valence_core_macros::Decode
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

/// Contains constants for every vanilla packet ID.
pub mod packet_id {
    include!(concat!(env!("OUT_DIR"), "/packet_id.rs"));
}

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use bytes::BytesMut;

    use super::*;
    use crate::protocol::decode::PacketDecoder;
    use crate::protocol::encode::PacketEncoder;

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet(id = 1)]
    struct RegularStruct {
        foo: i32,
        bar: bool,
        baz: f64,
    }

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet(id = 2)]
    struct UnitStruct;

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet(id = 3)]
    struct EmptyStruct {}

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet(id = 4)]
    struct TupleStruct(i32, bool, f64);

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet(id = 5)]
    struct StructWithGenerics<'z, T = ()> {
        foo: &'z str,
        bar: T,
    }

    #[derive(Encode, Decode, Packet, Debug)]
    #[packet(id = 6)]
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

    use crate::block_pos::BlockPos;
    use crate::hand::Hand;
    use crate::ident::Ident;
    use crate::item::{ItemKind, ItemStack};
    use crate::protocol::var_int::VarInt;
    use crate::protocol::var_long::VarLong;
    use crate::text::{Text, TextFormat};

    #[cfg(feature = "encryption")]
    const CRYPT_KEY: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

    #[derive(PartialEq, Debug, Encode, Decode, Packet)]
    #[packet(id = 42)]
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

        let pkt = frame.decode::<TestPacket>().unwrap();

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
