use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::QUERY_PONG_S2C, state = PacketState::Status)]
pub struct QueryPongS2c {
    pub payload: u64,
}
