use std::borrow::Cow;

use uuid::Uuid;

use crate::text::Text;
use crate::types::MessageSignature;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct ChatMessageS2c<'a> {
    pub sender: Uuid,
    pub index: VarInt,
    pub message_signature: Option<&'a [u8; 256]>,
    pub message: &'a str,
    pub time_stamp: u64,
    pub salt: u64,
    pub previous_messages: Vec<MessageSignature<'a>>,
    pub unsigned_content: Option<Cow<'a, Text>>,
    pub filter_type: MessageFilterType,
    pub chat_type: VarInt,
    pub network_name: Cow<'a, Text>,
    pub network_target_name: Option<Cow<'a, Text>>,
}

#[derive(Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum MessageFilterType {
    PassThrough,
    FullyFiltered,
    PartiallyFiltered { mask: Vec<u64> },
}
