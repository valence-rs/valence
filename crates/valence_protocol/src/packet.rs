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

/// Defines an enum of packets and implements `Packet` for each.
macro_rules! packet_group {
    (
        $(#[$attrs:meta])*
        $enum_name:ident<$enum_life:lifetime> {
            $($packet_id:literal = $packet:ident $(<$life:lifetime>)?),* $(,)?
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

            impl<$enum_life> crate::Packet<$enum_life> for $packet$(<$life>)? {
                const PACKET_ID: i32 = $packet_id;

                fn packet_id(&self) -> i32 {
                    $packet_id
                }

                #[allow(unused_imports)]
                fn encode_packet(&self, mut w: impl std::io::Write) -> crate::Result<()> {
                    use ::valence_protocol::__private::{Encode, Context, VarInt};

                    VarInt($packet_id)
                        .encode(&mut w)
                        .context("failed to encode packet ID")?;

                    self.encode(w)
                }

                #[allow(unused_imports)]
                fn decode_packet(r: &mut &$enum_life [u8]) -> ::valence_protocol::__private::Result<Self> {
                    use ::valence_protocol::__private::{Decode, Context, VarInt, ensure};

                    let id = VarInt::decode(r).context("failed to decode packet ID")?.0;
                    ensure!(id == $packet_id, "unexpected packet ID {} (expected {})", id, $packet_id);

                    Self::decode(r)
                }
            }
        )*

        impl<$enum_life> crate::Packet<$enum_life> for $enum_name<$enum_life> {
            fn packet_id(&self) -> i32 {
                match self {
                    $(
                        Self::$packet(_) => <$packet as crate::Packet>::PACKET_ID,
                    )*
                }
            }

            fn encode_packet(&self, mut w: impl std::io::Write) -> crate::Result<()> {
                use crate::Encode;
                use crate::var_int::VarInt;

                match self {
                    $(
                        Self::$packet(pkt) => {
                            VarInt(<$packet as crate::Packet>::PACKET_ID).encode(&mut w)?;
                            pkt.encode(w)?;
                        }
                    )*
                }

                Ok(())
            }

            fn decode_packet(r: &mut &$enum_life [u8]) -> crate::Result<Self> {
                use crate::Decode;
                use crate::var_int::VarInt;

                let id = VarInt::decode(r)?.0;
                Ok(match id {
                    $(
                        <$packet as crate::Packet>::PACKET_ID =>
                            Self::$packet($packet::decode(r)?),
                    )*
                    id => anyhow::bail!("unknown packet ID {} while decoding {}", id, stringify!($enum_name)),
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
            $($packet_id:literal = $packet:ident),* $(,)?
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

            impl crate::Packet<'_> for $packet {
                const PACKET_ID: i32 = $packet_id;

                fn packet_id(&self) -> i32 {
                    $packet_id
                }

                #[allow(unused_imports)]
                fn encode_packet(&self, mut w: impl std::io::Write) -> crate::Result<()> {
                    use ::valence_protocol::__private::{Encode, Context, VarInt};

                    VarInt($packet_id)
                        .encode(&mut w)
                        .context("failed to encode packet ID")?;

                    self.encode(w)
                }

                #[allow(unused_imports)]
                fn decode_packet(r: &mut &[u8]) -> ::valence_protocol::__private::Result<Self> {
                    use ::valence_protocol::__private::{Decode, Context, VarInt, ensure};

                    let id = VarInt::decode(r).context("failed to decode packet ID")?.0;
                    ensure!(id == $packet_id, "unexpected packet ID {} (expected {})", id, $packet_id);

                    Self::decode(r)
                }
            }
        )*

        impl crate::Packet<'_> for $enum_name {
            fn packet_id(&self) -> i32 {
                match self {
                    $(
                        Self::$packet(_) => <$packet as crate::Packet>::PACKET_ID,
                    )*
                }
            }

            fn encode_packet(&self, mut w: impl std::io::Write) -> crate::Result<()> {
                use crate::Encode;
                use crate::var_int::VarInt;

                match self {
                    $(
                        Self::$packet(pkt) => {
                            VarInt(<$packet as crate::Packet>::PACKET_ID).encode(&mut w)?;
                            pkt.encode(w)?;
                        }
                    )*
                }

                Ok(())
            }

            fn decode_packet(r: &mut &[u8]) -> crate::Result<Self> {
                use crate::Decode;
                use crate::var_int::VarInt;

                let id = VarInt::decode(r)?.0;
                Ok(match id {
                    $(
                        <$packet as crate::Packet>::PACKET_ID =>
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
