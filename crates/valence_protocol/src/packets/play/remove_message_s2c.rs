use super::chat_message_s2c::MessageSignature;
use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct RemoveMessageS2c<'a> {
    pub signature: MessageSignature<'a>,
}
