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
pub mod message_signature;
pub mod raw;
pub mod var_int;
pub mod var_long;

use std::io::Write;

pub use valence_core_macros::{Decode, Encode, Packet};

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
/// use valence_core::packet::Encode;
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
/// use valence_core::packet::Decode;
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
/// [macro]: valence_core_macros::Decode
/// [`VarInt`]: var_int::VarInt
pub trait Decode<'a>: Sized {
    /// Reads this object from the provided byte slice.
    ///
    /// Implementations of `Decode` are expected to shrink the slice from the
    /// front as bytes are read.
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self>;
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
/// type must implement [`Encode`], [`Decode`], and [`std::fmt::Debug`].
///
/// ```
/// use valence_core::packet::{Decode, Encode, Packet};
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
/// [macro]: valence_core_macros::Packet
/// [`VarInt`]: var_int::VarInt
pub trait Packet<'a>: Sized + std::fmt::Debug {
    /// The packet returned by [`Self::packet_id`]. If the packet ID is not
    /// statically known, then a negative value is used instead.
    const PACKET_ID: i32 = -1;
    /// Returns the ID of this packet.
    fn packet_id(&self) -> i32;
    /// Returns the name of this packet, typically without whitespace or
    /// additional formatting.
    fn packet_name(&self) -> &str;
    /// Like [`Encode::encode`], but a leading [`VarInt`] packet ID must be
    /// written by this method first.
    ///
    /// [`VarInt`]: var_int::VarInt
    fn encode_packet(&self, w: impl Write) -> anyhow::Result<()>;
    /// Like [`Decode::decode`], but a leading [`VarInt`] packet ID must be read
    /// by this method first.
    ///
    /// [`VarInt`]: var_int::VarInt
    fn decode_packet(r: &mut &'a [u8]) -> anyhow::Result<Self>;
}

/// Defines an enum of packets and implements [`Packet`] for each.
macro_rules! packet_group {
    (
        $(#[$attrs:meta])*
        $enum_name:ident<$enum_life:lifetime> {
            $($packet:ident $(<$life:lifetime>)?),* $(,)?
        }
    ) => {
        $(#[$attrs])*
        pub enum $enum_name<$enum_life> {
            $(
                $packet($packet $(<$life>)?),
            )*
        }

        $(
            impl<$enum_life> From<$packet $(<$life>)?> for $enum_name<$enum_life> {
                fn from(p: $packet $(<$life>)?) -> Self {
                    Self::$packet(p)
                }
            }

            impl<$enum_life> $crate::packet::Packet<$enum_life> for $packet$(<$life>)? {
                const PACKET_ID: i32 = $crate::packet::id::$packet;

                fn packet_id(&self) -> i32 {
                    Self::PACKET_ID
                }

                fn packet_name(&self) -> &str {
                    stringify!($packet)
                }

                #[allow(unused_imports)]
                fn encode_packet(&self, mut w: impl std::io::Write) -> $crate::__private::Result<()> {
                    use $crate::__private::*;

                    VarInt(Self::PACKET_ID)
                        .encode(&mut w)
                        .context("failed to encode packet ID")?;

                    self.encode(w)
                }

                #[allow(unused_imports)]
                fn decode_packet(r: &mut &$enum_life [u8]) -> $crate::__private::Result<Self> {
                    use $crate::__private::*;

                    let id = VarInt::decode(r).context("failed to decode packet ID")?.0;
                    ensure!(id == Self::PACKET_ID, "unexpected packet ID {} (expected {})", id, Self::PACKET_ID);

                    Self::decode(r)
                }
            }
        )*

        impl<$enum_life> $crate::packet::Packet<$enum_life> for $enum_name<$enum_life> {
            fn packet_id(&self) -> i32 {
                match self {
                    $(
                        Self::$packet(_) => <$packet as $crate::packet::Packet>::PACKET_ID,
                    )*
                }
            }

            fn packet_name(&self) -> &str {
                match self {
                    $(
                        Self::$packet(pkt) => pkt.packet_name(),
                    )*
                }
            }

            fn encode_packet(&self, mut w: impl std::io::Write) -> $crate::__private::Result<()> {
                use $crate::__private::*;

                match self {
                    $(
                        Self::$packet(pkt) => {
                            VarInt(<$packet as Packet>::PACKET_ID).encode(&mut w)?;
                            pkt.encode(w)?;
                        }
                    )*
                }

                Ok(())
            }

            fn decode_packet(r: &mut &$enum_life [u8]) -> $crate::__private::Result<Self> {
                use $crate::__private::*;

                let id = VarInt::decode(r)?.0;
                Ok(match id {
                    $(
                        <$packet as Packet>::PACKET_ID =>
                            Self::$packet($packet::decode(r)?),
                    )*
                    id => bail!("unknown packet ID {} while decoding {}", id, stringify!($enum_name)),
                })
            }
        }

        impl<$enum_life> std::fmt::Debug for $enum_name<$enum_life> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        Self::$packet(pkt) => pkt.fmt(f),
                    )*
                }
            }
        }
    };
    // No lifetime on the enum in this case.
    (
        $(#[$attrs:meta])*
        $enum_name:ident {
            $($packet:ident),* $(,)?
        }
    ) => {
        $(#[$attrs])*
        pub enum $enum_name {
            $(
                $packet($packet),
            )*
        }

        $(
            impl From<$packet> for $enum_name {
                fn from(p: $packet) -> Self {
                    Self::$packet(p)
                }
            }

            impl $crate::__private::Packet<'_> for $packet {
                const PACKET_ID: i32 = $crate::packet::id::$packet;

                fn packet_id(&self) -> i32 {
                    Self::PACKET_ID
                }

                fn packet_name(&self) -> &str {
                    stringify!($packet)
                }

                #[allow(unused_imports)]
                fn encode_packet(&self, mut w: impl std::io::Write) -> $crate::__private::Result<()> {
                    use $crate::__private::*;

                    VarInt(Self::PACKET_ID)
                        .encode(&mut w)
                        .context("failed to encode packet ID")?;

                    self.encode(w)
                }

                #[allow(unused_imports)]
                fn decode_packet(r: &mut &[u8]) -> $crate::__private::Result<Self> {
                    use $crate::__private::*;

                    let id = VarInt::decode(r).context("failed to decode packet ID")?.0;
                    ensure!(id == Self::PACKET_ID, "unexpected packet ID {} (expected {})", id, Self::PACKET_ID);

                    Self::decode(r)
                }
            }
        )*

        impl $crate::__private::Packet<'_> for $enum_name {
            fn packet_id(&self) -> i32 {
                use $crate::__private::*;

                match self {
                    $(
                        Self::$packet(_) => <$packet as Packet>::PACKET_ID,
                    )*
                }
            }

            fn packet_name(&self) -> &str {
                match self {
                    $(
                        Self::$packet(pkt) => pkt.packet_name(),
                    )*
                }
            }

            fn encode_packet(&self, mut w: impl std::io::Write) -> $crate::__private::Result<()> {
                use $crate::__private::*;

                match self {
                    $(
                        Self::$packet(pkt) => {
                            VarInt(<$packet as Packet>::PACKET_ID).encode(&mut w)?;
                            pkt.encode(w)?;
                        }
                    )*
                }

                Ok(())
            }

            fn decode_packet(r: &mut &[u8]) -> $crate::__private::Result<Self> {
                use $crate::__private::*;

                let id = VarInt::decode(r)?.0;
                Ok(match id {
                    $(
                        <$packet as Packet>::PACKET_ID =>
                            Self::$packet($packet::decode(r)?),
                    )*
                    id => anyhow::bail!("unknown packet ID {} while decoding {}", id, stringify!($enum_name)),
                })
            }
        }

        impl std::fmt::Debug for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        Self::$packet(pkt) => pkt.fmt(f),
                    )*
                }
            }
        }
    }
}

pub mod c2s;
pub mod s2c;

/// Contains the packet ID for every packet. Because the constants are private
/// to the crate, the compiler will yell at us when we forget to use one.
mod id {
    include!(concat!(env!("OUT_DIR"), "/packet_id.rs"));
}

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use bytes::BytesMut;

    use super::*;
    use crate::packet::c2s::play::{C2sPlayPacket, HandSwingC2s};
    use crate::packet::decode::{decode_packet, PacketDecoder};
    use crate::packet::encode::PacketEncoder;

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

    #[allow(unconditional_recursion, clippy::extra_unused_type_parameters)]
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
    use crate::hand::Hand;
    use crate::ident::Ident;
    use crate::item::{ItemKind, ItemStack};
    use crate::packet::var_int::VarInt;
    use crate::packet::var_long::VarLong;
    use crate::text::{Text, TextFormat};

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
    fn packets_round_trip() -> anyhow::Result<()> {
        let mut buf = BytesMut::new();

        let mut enc = PacketEncoder::new();

        enc.append_packet(&TestPacket::new("first")).unwrap();
        #[cfg(feature = "compression")]
        enc.set_compression(Some(0));
        enc.append_packet(&TestPacket::new("second")).unwrap();
        buf.unsplit(enc.take());
        #[cfg(feature = "encryption")]
        enc.enable_encryption(&CRYPT_KEY)?;
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
        dec.enable_encryption(&CRYPT_KEY)?;

        check_test_packet(&mut dec, "fourth");
        check_test_packet(&mut dec, "third");

        Ok(())
    }
}
