use crate::{Decode, Encode, Packet, PacketState};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Status, name = "PING_RESULT_S2C")]
pub struct QueryPongS2c {
    pub payload: u64,
}
