use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x11]
pub struct KeepAliveC2s {
    pub id: u64,
}
