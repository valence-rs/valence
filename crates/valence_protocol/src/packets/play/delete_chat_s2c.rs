use super::player_chat_s2c::MessageSignature;
use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct DeleteChatS2c<'a> {
    pub signature: MessageSignature<'a>,
}
