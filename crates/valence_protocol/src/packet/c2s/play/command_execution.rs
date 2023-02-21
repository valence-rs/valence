use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
#[packet_id = 0x04]
pub struct CommandExecutionC2s<'a> {
    pub command: &'a str,
    pub timestamp: u64,
    pub salt: u64,
    pub argument_signatures: Vec<CommandArgumentSignature<'a>>,
    pub message_count: VarInt,
    //// This is a bitset of 20; each bit represents one
    //// of the last 20 messages received and whether or not
    //// the message was acknowledged by the client
    pub acknowledgement: [u8; 3],
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct CommandArgumentSignature<'a> {
    pub argument_name: &'a str,
    pub signature: &'a [u8; 256],
}
