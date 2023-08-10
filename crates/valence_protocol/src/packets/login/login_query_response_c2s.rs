use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOGIN_QUERY_RESPONSE_C2S, state = PacketState::Login)]
pub struct LoginQueryResponseC2s<'a> {
    pub message_id: VarInt,
    pub data: Option<RawBytes<'a>>,
}
