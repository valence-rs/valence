//! Packet definitions and related types.
//!
//! See <https://wiki.vg/Protocol> for more packet documentation.

#![macro_use]

use std::fmt;
use std::io::Write;

use anyhow::{bail, ensure, Context};
use bitvec::prelude::BitVec;
use num::{One, Zero};
use paste::paste;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vek::Vec3;

// use {def_bitfield, def_enum, def_struct};
use crate::block_pos::BlockPos;
use crate::ident::Ident;
use crate::nbt::Compound;
use crate::protocol::{
    BoundedArray, BoundedInt, BoundedString, ByteAngle, Decode, Encode, NbtBridge, RawBytes,
    VarInt, VarLong,
};
use crate::slot::Slot;
use crate::text::Text;

/// Provides the name of a packet for debugging purposes.
pub trait PacketName {
    /// The name of this packet.
    fn packet_name(&self) -> &'static str;
}

/// Trait for types that can be written to the Minecraft protocol as a complete
/// packet.
///
/// A complete packet is one that starts with a `VarInt` packet ID, followed by
/// the body of the packet.
pub trait EncodePacket: PacketName + fmt::Debug {
    /// Writes a packet to the Minecraft protocol, including its packet ID.
    fn encode_packet(&self, w: &mut impl Write) -> anyhow::Result<()>;
}

/// Trait for types that can be read from the Minecraft protocol as a complete
/// packet.
///
/// A complete packet is one that starts with a `VarInt` packet ID, followed by
/// the body of the packet.
pub trait DecodePacket: Sized + PacketName + fmt::Debug {
    /// Reads a packet from the Minecraft protocol, including its packet ID.
    fn decode_packet(r: &mut &[u8]) -> anyhow::Result<Self>;
}

/// Defines a struct which implements [`Encode`] and [`Decode`].
///
/// The fields of the struct are encoded and decoded in the order they are
/// defined.
macro_rules! def_struct {
    (
        $(#[$struct_attrs:meta])*
        $name:ident {
            $(
                $(#[$field_attrs:meta])*
                $field:ident: $typ:ty
            ),* $(,)?
        }
    ) => {
        #[derive(Clone, Debug)]
        $(#[$struct_attrs])*
        pub struct $name {
            $(
                $(#[$field_attrs])*
                pub $field: $typ,
            )*
        }

        impl Encode for $name {
            fn encode(&self, _w: &mut impl Write) -> anyhow::Result<()> {
                $(
                    Encode::encode(&self.$field, _w)
                        .context(concat!("failed to write field `", stringify!($field), "` from struct `", stringify!($name), "`"))?;
                )*
                Ok(())
            }
        }

        impl Decode for $name {
            fn decode(_r: &mut &[u8]) -> anyhow::Result<Self> {
                $(
                    let $field: $typ = Decode::decode(_r)
                        .context(concat!("failed to read field `", stringify!($field), "` from struct `", stringify!($name), "`"))?;
                )*

                Ok(Self {
                    $(
                        $field,
                    )*
                })
            }
        }

        // TODO: https://github.com/rust-lang/rust/issues/48214
        //impl Copy for $name
        //where
        //    $(
        //        $typ: Copy
        //    )*
        //{}
    }
}

/// Defines an enum which implements [`Encode`] and [`Decode`].
///
/// The enum tag is encoded and decoded first, followed by the appropriate
/// variant.
macro_rules! def_enum {
    (
        $(#[$enum_attrs:meta])*
        $name:ident: $tag_ty:ty {
            $(
                $(#[$variant_attrs:meta])*
                $variant:ident$(: $typ:ty)? = $lit:literal
            ),* $(,)?
        }
    ) => {
        #[derive(Clone, Debug)]
        $(#[$enum_attrs])*
        pub enum $name {
            $(
                $(#[$variant_attrs])*
                $variant$(($typ))?,
            )*
        }

        impl Encode for $name {
            fn encode(&self, _w: &mut impl Write) -> anyhow::Result<()> {
                match self {
                    $(
                        if_typ_is_empty_pat!($($typ)?, $name::$variant, $name::$variant(val)) => {
                            <$tag_ty>::encode(&$lit.into(), _w)
                                .context(concat!("failed to write enum tag for `", stringify!($name), "`"))?;

                            if_typ_is_empty_expr!($($typ)?, Ok(()), {
                                Encode::encode(val, _w)
                                    .context(concat!("failed to write variant `", stringify!($variant), "` from enum `", stringify!($name), "`"))
                            })
                        },
                    )*

                    // Need this because references to uninhabited enums are considered inhabited.
                    #[allow(unreachable_patterns)]
                    _ => unreachable!("uninhabited enum?")
                }
            }
        }

        impl Decode for $name {
            fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
                let tag_ctx = concat!("failed to read enum tag for `", stringify!($name), "`");
                let tag = <$tag_ty>::decode(r).context(tag_ctx)?.into();
                match tag {
                    $(
                        $lit => {
                            if_typ_is_empty_expr!($($typ)?, Ok($name::$variant), {
                                $(
                                    let res: $typ = Decode::decode(r)
                                        .context(concat!("failed to read variant `", stringify!($variant), "` from enum `", stringify!($name), "`"))?;
                                    Ok($name::$variant(res))
                                )?
                            })
                        }
                    )*
                    _ => bail!(concat!("bad tag value for enum `", stringify!($name), "`"))
                }
            }
        }
    }
}

macro_rules! if_typ_is_empty_expr {
    (, $t:expr, $f:expr) => {
        $t
    };
    ($typ:ty, $t:expr, $f:expr) => {
        $f
    };
}

macro_rules! if_typ_is_empty_pat {
    (, $t:pat, $f:pat) => {
        $t
    };
    ($typ:ty, $t:pat, $f:pat) => {
        $f
    };
}

/// Defines a bitfield struct which implements [`Encode`] and [`Decode`].
macro_rules! def_bitfield {
    (
        $(#[$struct_attrs:meta])*
        $name:ident: $inner_ty:ty {
            $(
                $(#[$bit_attrs:meta])*
                $bit:ident = $offset:literal
            ),* $(,)?
        }
    ) => {
        #[derive(Clone, Copy, PartialEq, Eq)]
        $(#[$struct_attrs])*
        pub struct $name($inner_ty);

        impl $name {
            pub fn new(
                $(
                    $bit: bool,
                )*
            ) -> Self {
                let mut res = Self(Default::default());
                paste! {
                    $(
                        res = res.[<set_ $bit:snake>]($bit);
                    )*
                }
                res
            }

            paste! {
                $(
                    #[doc = "Gets the " $bit " bit on this bitfield.\n"]
                    $(#[$bit_attrs])*
                    pub fn $bit(self) -> bool {
                        self.0 & <$inner_ty>::one() << <$inner_ty>::from($offset) != <$inner_ty>::zero()
                    }

                    #[doc = "Sets the " $bit " bit on this bitfield.\n"]
                    $(#[$bit_attrs])*
                    #[must_use]
                    pub fn [<set_ $bit:snake>](self, $bit: bool) -> Self {
                        let mask = <$inner_ty>::one() << <$inner_ty>::from($offset);
                        if $bit {
                            Self(self.0 | mask)
                        } else {
                            Self(self.0 & !mask)
                        }
                    }
                )*
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut s = f.debug_struct(stringify!($name));
                paste! {
                    $(
                        s.field(stringify!($bit), &self. $bit());
                    )*
                }
                s.finish()
            }
        }

        impl Encode for $name {
            fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
                self.0.encode(w)
            }
        }

        impl Decode for $name {
            fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
                <$inner_ty>::decode(r).map(Self)
            }
        }
    }
}

/// Defines an enum of packets.
///
/// An impl for [`EncodePacket`] and [`DecodePacket`] is defined for each
/// supplied packet.
macro_rules! def_packet_group {
    (
        $(#[$attrs:meta])*
        $group_name:ident {
            $($packet:ident = $id:literal),* $(,)?
        }
    ) => {
        #[derive(Clone)]
        $(#[$attrs])*
        pub enum $group_name {
            $($packet($packet)),*
        }

        $(
            impl From<$packet> for $group_name {
                fn from(p: $packet) -> Self {
                    Self::$packet(p)
                }
            }

            impl PacketName for $packet {
                fn packet_name(&self) -> &'static str {
                    stringify!($packet)
                }
            }

            impl EncodePacket for $packet {
                fn encode_packet(&self, w: &mut impl Write) -> anyhow::Result<()> {
                    VarInt($id).encode(w).context("failed to write packet ID")?;
                    self.encode(w)
                }
            }

            impl DecodePacket for $packet {
                fn decode_packet(r: &mut &[u8]) -> anyhow::Result<Self> {
                    let packet_id = VarInt::decode(r).context("failed to read packet ID")?.0;

                    ensure!(
                        $id == packet_id,
                        "bad packet ID (expected {}, got {packet_id}",
                        $id
                    );
                    Self::decode(r)
                }
            }
        )*

        impl PacketName for $group_name {
            fn packet_name(&self) -> &'static str {
                match self {
                    $(
                        Self::$packet(pkt) => pkt.packet_name(),
                    )*
                }
            }
        }

        impl DecodePacket for $group_name {
            fn decode_packet(r: &mut &[u8]) -> anyhow::Result<Self> {
                let packet_id = VarInt::decode(r)
                    .context(concat!("failed to read ", stringify!($group_name), " packet ID"))?.0;

                match packet_id {
                    $(
                        $id => {
                            let pkt = $packet::decode(r)?;
                            Ok(Self::$packet(pkt))
                        }
                    )*
                    id => bail!(concat!("unknown ", stringify!($group_name), " packet ID {}"), id),
                }
            }
        }

        impl EncodePacket for $group_name {
            fn encode_packet(&self, w: &mut impl Write) -> anyhow::Result<()> {
                match self {
                    $(
                        Self::$packet(pkt) => {
                            VarInt($id)
                                .encode(w)
                                .context(concat!(
                                    "failed to write ",
                                    stringify!($group_name),
                                    " packet ID for ",
                                    stringify!($packet_name)
                                ))?;
                            pkt.encode(w)
                        }
                    )*
                }
            }
        }

        impl fmt::Debug for $group_name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut t = f.debug_tuple(stringify!($group_name));
                match self {
                    $(
                        Self::$packet(pkt) => t.field(pkt),
                    )*
                };
                t.finish()
            }
        }
    }
}

// Must be below the macro_rules!.
pub mod c2s;
pub mod s2c;

def_struct! {
    #[derive(PartialEq, Serialize, Deserialize)]
    Property {
        name: String,
        value: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>
    }
}

def_struct! {
    PublicKeyData {
        timestamp: u64,
        public_key: Vec<u8>,
        signature: Vec<u8>,
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;

    def_struct! {
        TestPacket {
            first: String,
            second: Vec<u16>,
            third: u64
        }
    }

    def_packet_group! {
        TestPacketGroup {
            TestPacket = 12345,
        }
    }
}
