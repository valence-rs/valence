use std::borrow::Cow;
use std::io::Write;

use uuid::Uuid;
use valence_text::Text;

use crate::{Bounded, Decode, Encode, Packet, VarInt};

#[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
pub struct ChatMessageS2c<'a> {
    pub sender: Uuid,
    pub index: VarInt,
    pub message_signature: Option<&'a [u8; 256]>,
    pub message: Bounded<&'a str, 256>,
    pub timestamp: u64,
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

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MessageSignature<'a> {
    ByIndex(i32),
    BySignature(&'a [u8; 256]),
}

impl<'a> Encode for MessageSignature<'a> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        match self {
            MessageSignature::ByIndex(index) => VarInt(index + 1).encode(&mut w)?,
            MessageSignature::BySignature(signature) => {
                VarInt(0).encode(&mut w)?;
                signature.encode(&mut w)?;
            }
        }

        Ok(())
    }
}

impl<'a> Decode<'a> for MessageSignature<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let index = VarInt::decode(r)?.0.saturating_sub(1);

        if index == -1 {
            Ok(MessageSignature::BySignature(<&[u8; 256]>::decode(r)?))
        } else {
            Ok(MessageSignature::ByIndex(index))
        }
    }
}
