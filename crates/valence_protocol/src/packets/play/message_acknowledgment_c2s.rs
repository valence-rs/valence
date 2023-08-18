use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct MessageAcknowledgmentC2s {
    pub message_count: VarInt,
}
