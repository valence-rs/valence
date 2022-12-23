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

        impl<$enum_life> crate::Encode for $enum_name<$enum_life> {
            fn encode(&self, mut w: impl std::io::Write) -> crate::Result<()> {
                use crate::DerivedPacketEncode;
                use crate::var_int::VarInt;

                match self {
                    $(
                        Self::$packet(pkt) => {
                            VarInt($packet::ID).encode(&mut w)?;
                            pkt.encode_without_id(w)?;
                        }
                    )*
                }

                Ok(())
            }
        }

        impl<$enum_life> crate::Decode<$enum_life> for $enum_name<$enum_life> {
            fn decode(r: &mut &$enum_life [u8]) -> crate::Result<Self> {
                use crate::DerivedPacketDecode;
                use crate::var_int::VarInt;

                let id = VarInt::decode(r)?.0;
                Ok(match id {
                    $(
                        $packet::ID => Self::$packet($packet::decode_without_id(r)?),
                    )*
                    id => anyhow::bail!("unknown packet id {}", id),
                })
            }
        }

        impl<$enum_life> crate::Packet for $enum_name<$enum_life> {
            fn packet_name(&self) -> &'static str {
                match self {
                    $(
                        Self::$packet(pkt) => pkt.packet_name(),
                    )*
                }
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

        impl crate::Encode for $enum_name {
            fn encode(&self, mut w: impl std::io::Write) -> crate::Result<()> {
                use crate::DerivedPacketEncode;
                use crate::var_int::VarInt;

                match self {
                    $(
                        Self::$packet(pkt) => {
                            VarInt($packet::ID).encode(&mut w)?;
                            pkt.encode_without_id(w)?;
                        }
                    )*
                }

                Ok(())
            }
        }

        impl crate::Decode<'_> for $enum_name {
            fn decode(r: &mut &[u8]) -> crate::Result<Self> {
                use crate::DerivedPacketDecode;
                use crate::var_int::VarInt;

                let id = VarInt::decode(r)?.0;
                Ok(match id {
                    $(
                        $packet::ID => Self::$packet($packet::decode_without_id(r)?),
                    )*
                    id => anyhow::bail!("unknown packet id {}", id),
                })
            }
        }

        impl crate::Packet for $enum_name {
            fn packet_name(&self) -> &'static str {
                match self {
                    $(
                        Self::$packet(pkt) => pkt.packet_name(),
                    )*
                }
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
