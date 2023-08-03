use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOGIN_COMPRESSION_S2C, state = PacketState::Login)]
pub struct LoginCompressionS2c {
    pub threshold: VarInt,
}
