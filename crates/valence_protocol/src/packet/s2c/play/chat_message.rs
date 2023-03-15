use std::borrow::Cow;
use std::io::Write;

use uuid::Uuid;

use crate::text::Text;
use crate::types::MessageSignature;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, PartialEq, Debug)]
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

impl<'a> Encode for ChatMessageS2c<'a> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        self.sender.encode(&mut w)?;
        self.index.encode(&mut w)?;
        self.message_signature.encode(&mut w)?;
        self.message.encode(&mut w)?;
        self.time_stamp.encode(&mut w)?;
        self.salt.encode(&mut w)?;
        self.previous_messages.encode(&mut w)?;
        self.unsigned_content.encode(&mut w)?;
        self.filter_type.encode(&mut w)?;
        self.chat_type.encode(&mut w)?;
        self.network_name.encode(&mut w)?;
        self.network_target_name.encode(&mut w)?;

        Ok(())
    }
}

impl<'a> Decode<'a> for ChatMessageS2c<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        Ok(Self {
            sender: Uuid::decode(r)?,
            index: VarInt::decode(r)?,
            message_signature: Option::<&'a [u8; 256]>::decode(r)?,
            message: <&str>::decode(r)?,
            time_stamp: u64::decode(r)?,
            salt: u64::decode(r)?,
            previous_messages: Vec::<MessageSignature>::decode(r)?,
            unsigned_content: Option::<Cow<'a, Text>>::decode(r)?,
            filter_type: MessageFilterType::decode(r)?,
            chat_type: VarInt::decode(r)?,
            network_name: <Cow<'a, Text>>::decode(r)?,
            network_target_name: Option::<Cow<'a, Text>>::decode(r)?,
        })
    }
}
