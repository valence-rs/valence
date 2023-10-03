use crate::{Bounded, Decode, Encode, FixedBitSet, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct CommandExecutionC2s<'a> {
    pub command: Bounded<&'a str, 256>,
    pub timestamp: u64,
    pub salt: u64,
    pub argument_signatures: Bounded<Vec<CommandArgumentSignature<'a>>, 8>,
    pub message_index: VarInt,
    //// This is a bitset of 20; each bit represents one
    //// of the last 20 messages received and whether or not
    //// the message was acknowledged by the client
    pub acknowledgement: FixedBitSet<20, 3>,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct CommandArgumentSignature<'a> {
    pub argument_name: Bounded<&'a str, 16>,
    pub signature: &'a [u8; 256],
}
