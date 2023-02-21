use crate::{Decode, DecodePacket, Encode, EncodePacket, VarInt};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x00]
pub struct HandshakeC2s<'a> {
    pub protocol_version: VarInt,
    pub server_address: &'a str,
    pub server_port: u16,
    pub next_state: NextState,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub enum NextState {
    #[tag = 1]
    Status,
    #[tag = 2]
    Login,
}
