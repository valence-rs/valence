pub mod decode;
pub mod encode;

use std::io::Write;

use anyhow::Context;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::*;
pub use valence_packet_macros::Packet;

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
    /// The side this packet is intended for
    const SIDE: PacketSide;
    /// The state which this packet is used
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

/// The side a packet is intended for
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PacketSide {
    /// Server -> Client
    Clientbound,
    /// Client -> Server
    Serverbound,
}

/// The state which a packet is used
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PacketState {
    Handshaking,
    Status,
    Login,
    Play,
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

    use valence_core::block_pos::BlockPos;
    use valence_core::hand::Hand;
    use valence_core::ident::Ident;
    use valence_core::item::{ItemKind, ItemStack};
    use valence_core::protocol::var_int::VarInt;
    use valence_core::protocol::var_long::VarLong;
    use valence_core::text::{IntoText, Text};

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
