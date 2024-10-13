use crate::{Decode, Encode, Packet, PacketState, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Login)]
// Optionally sent by the server to the client to enable compression for the connection.
pub struct LoginCompressionS2c {
    pub threshold: VarInt,
}
