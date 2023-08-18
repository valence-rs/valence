use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Status)]
pub struct QueryPingC2s {
    pub payload: u64,
}
