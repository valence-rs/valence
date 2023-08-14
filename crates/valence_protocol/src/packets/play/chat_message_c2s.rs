use crate::FixedBitSet;

use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CHAT_MESSAGE_C2S)]
pub struct ChatMessageC2s<'a> {
    pub message: Bounded<&'a str, 256>,
    pub timestamp: u64,
    pub salt: u64,
    pub signature: Option<&'a [u8; 256]>,
    pub message_count: VarInt,
    // This is a bitset of 20; each bit represents one
    // of the last 20 messages received and whether or not
    // the message was acknowledged by the client
    pub acknowledgement: FixedBitSet<20, 3>,
}
