use super::chat_message_s2c::MessageSignature;
use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::REMOVE_MESSAGE_S2C)]
pub struct RemoveMessageS2c<'a> {
    pub signature: MessageSignature<'a>,
}
