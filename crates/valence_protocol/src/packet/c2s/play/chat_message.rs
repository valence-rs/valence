use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct ChatMessageC2s<'a> {
    pub message: &'a str,
    pub timestamp: u64,
    pub salt: u64,
    pub signature: Option<&'a [u8; 256]>,
    pub message_count: VarInt,
    // This is a bitset of 20; each bit represents one
    // of the last 20 messages received and whether or not
    // the message was acknowledged by the client
    pub acknowledgement: [u8; 3],
}
