use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x1f]
pub struct KeepAliveS2c {
    pub id: u64,
}
