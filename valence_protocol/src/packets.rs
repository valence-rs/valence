//! Contains client-to-server ([`c2s`]) and server-to-client ([`s2c`]) packets
//! for the current version of the game.
//!
//! If the packets as defined do not meet your needs, consider using the tools
//! in this library to redefine the packets yourself.

pub use c2s::handshake::C2sHandshakePacket;
pub use c2s::login::C2sLoginPacket;
pub use c2s::play::C2sPlayPacket;
pub use c2s::status::C2sStatusPacket;
pub use s2c::login::S2cLoginPacket;
pub use s2c::play::S2cPlayPacket;
pub use s2c::status::S2cStatusPacket;

/// Defines an enum of packets.
macro_rules! packet_enum {
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
        )*

        impl<$enum_life> crate::EncodePacket for $enum_name<$enum_life> {
            fn encode_packet(&self, mut w: impl std::io::Write) -> crate::Result<()> {
                use crate::{Encode, VarInt};

                match self {
                    $(
                        Self::$packet(pkt) => {
                            VarInt(<$packet as crate::EncodePacket>::PACKET_ID).encode(&mut w)?;
                            pkt.encode(w)?;
                        }
                    )*
                }

                Ok(())
            }
        }

        impl<$enum_life> crate::DecodePacket<$enum_life> for $enum_name<$enum_life> {
            fn decode_packet(r: &mut &$enum_life [u8]) -> crate::Result<Self> {
                use crate::{Decode, VarInt};

                let id = VarInt::decode(r)?.0;
                Ok(match id {
                    $(
                        <$packet as crate::DecodePacket>::PACKET_ID =>
                            Self::$packet($packet::decode(r)?),
                    )*
                    id => anyhow::bail!("unknown packet id {}", id),
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
        )*

        impl crate::EncodePacket for $enum_name {
            fn encode_packet(&self, mut w: impl std::io::Write) -> crate::Result<()> {
                use crate::{Encode, VarInt};

                match self {
                    $(
                        Self::$packet(pkt) => {
                            VarInt(<$packet as crate::EncodePacket>::PACKET_ID).encode(&mut w)?;
                            pkt.encode(w)?;
                        }
                    )*
                }

                Ok(())
            }
        }

        impl crate::DecodePacket<'_> for $enum_name {
            fn decode_packet(r: &mut &[u8]) -> crate::Result<Self> {
                use crate::{Decode, VarInt};

                let id = VarInt::decode(r)?.0;
                Ok(match id {
                    $(
                        <$packet as crate::DecodePacket>::PACKET_ID =>
                            Self::$packet($packet::decode(r)?),
                    )*
                    id => anyhow::bail!("unknown packet id {}", id),
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
