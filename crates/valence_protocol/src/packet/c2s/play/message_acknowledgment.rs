use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x03]
pub struct MessageAcknowledgmentC2s {
    pub message_count: VarInt,
}
