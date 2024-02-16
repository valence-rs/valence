use crate::{Decode, Encode, Packet, PacketState, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Login)]
pub struct LoginCompressionS2c {
    pub threshold: VarInt,
}
