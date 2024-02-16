use crate::{Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct MessageAcknowledgmentC2s {
    pub message_count: VarInt,
}
