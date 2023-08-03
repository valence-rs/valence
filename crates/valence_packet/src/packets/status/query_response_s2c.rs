use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::QUERY_RESPONSE_S2C, state = PacketState::Status)]
pub struct QueryResponseS2c<'a> {
    pub json: &'a str,
}
