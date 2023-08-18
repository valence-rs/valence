use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Status)]
pub struct QueryPongS2c {
    pub payload: u64,
}
