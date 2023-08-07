use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::QUERY_REQUEST_C2S, state = PacketState::Status)]
pub struct QueryRequestC2s;
