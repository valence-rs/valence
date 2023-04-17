use crate::packet::message_signature::MessageSignature;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct RemoveMessageS2c<'a> {
    pub signature: MessageSignature<'a>,
}
