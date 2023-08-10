use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::MESSAGE_ACKNOWLEDGMENT_C2S)]

pub struct MessageAcknowledgmentC2s {
    pub message_count: VarInt,
}
