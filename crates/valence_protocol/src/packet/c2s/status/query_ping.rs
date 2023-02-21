use crate::{Decode, DecodePacket, Encode, EncodePacket};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x01]
pub struct QueryPingC2s {
    pub payload: u64,
}
