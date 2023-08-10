use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::QUERY_PING_C2S, state = PacketState::Status)]
pub struct QueryPingC2s {
    pub payload: u64,
}
