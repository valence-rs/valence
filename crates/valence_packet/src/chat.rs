use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::GAME_MESSAGE_S2C)]
pub struct GameMessageS2c<'a> {
    pub chat: Cow<'a, Text>,
    /// Whether the message is in the actionbar or the chat.
    pub overlay: bool,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct MessageSignature<'a> {
    pub message_id: i32,
    pub signature: Option<&'a [u8; 256]>,
}

impl<'a> Encode for MessageSignature<'a> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        VarInt(self.message_id + 1).encode(&mut w)?;

        match self.signature {
            None => {}
            Some(signature) => signature.encode(&mut w)?,
        }

        Ok(())
    }
}

impl<'a> Decode<'a> for MessageSignature<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let message_id = VarInt::decode(r)?.0 - 1; // TODO: this can underflow.

        let signature = if message_id == -1 {
            Some(<&[u8; 256]>::decode(r)?)
        } else {
            None
        };

        Ok(Self {
            message_id,
            signature,
        })
    }
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CHAT_MESSAGE_C2S)]
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

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::COMMAND_EXECUTION_C2S)]
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

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::MESSAGE_ACKNOWLEDGMENT_C2S)]

pub struct MessageAcknowledgmentC2s {
    pub message_count: VarInt,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_SESSION_C2S)]
pub struct PlayerSessionC2s<'a> {
    pub session_id: Uuid,
    // Public key
    pub expires_at: i64,
    pub public_key_data: &'a [u8],
    pub key_signature: &'a [u8],
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::REQUEST_COMMAND_COMPLETIONS_C2S)]
pub struct RequestCommandCompletionsC2s<'a> {
    pub transaction_id: VarInt,
    pub text: &'a str,
}

#[derive(Clone, PartialEq, Debug, Packet)]
#[packet(id = packet_id::CHAT_MESSAGE_S2C)]
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
    pub filter_type_bits: Option<u8>,
    pub chat_type: VarInt,
    pub network_name: Cow<'a, Text>,
    pub network_target_name: Option<Cow<'a, Text>>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum MessageFilterType {
    PassThrough,
    FullyFiltered,
    PartiallyFiltered,
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

        if self.filter_type == MessageFilterType::PartiallyFiltered {
            match self.filter_type_bits {
                // Filler data
                None => 0u8.encode(&mut w)?,
                Some(bits) => bits.encode(&mut w)?,
            }
        }

        self.chat_type.encode(&mut w)?;
        self.network_name.encode(&mut w)?;
        self.network_target_name.encode(&mut w)?;

        Ok(())
    }
}

impl<'a> Decode<'a> for ChatMessageS2c<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let sender = Uuid::decode(r)?;
        let index = VarInt::decode(r)?;
        let message_signature = Option::<&'a [u8; 256]>::decode(r)?;
        let message = <&str>::decode(r)?;
        let time_stamp = u64::decode(r)?;
        let salt = u64::decode(r)?;
        let previous_messages = Vec::<MessageSignature>::decode(r)?;
        let unsigned_content = Option::<Cow<'a, Text>>::decode(r)?;
        let filter_type = MessageFilterType::decode(r)?;

        let filter_type_bits = match filter_type {
            MessageFilterType::PartiallyFiltered => Some(u8::decode(r)?),
            _ => None,
        };

        let chat_type = VarInt::decode(r)?;
        let network_name = <Cow<'a, Text>>::decode(r)?;
        let network_target_name = Option::<Cow<'a, Text>>::decode(r)?;

        Ok(Self {
            sender,
            index,
            message_signature,
            message,
            time_stamp,
            salt,
            previous_messages,
            unsigned_content,
            filter_type,
            filter_type_bits,
            chat_type,
            network_name,
            network_target_name,
        })
    }
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CHAT_SUGGESTIONS_S2C)]
pub struct ChatSuggestionsS2c<'a> {
    pub action: ChatSuggestionsAction,
    pub entries: Cow<'a, [&'a str]>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum ChatSuggestionsAction {
    Add,
    Remove,
    Set,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::REMOVE_MESSAGE_S2C)]
pub struct RemoveMessageS2c<'a> {
    pub signature: MessageSignature<'a>,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::COMMAND_SUGGESTIONS_S2C)]
pub struct CommandSuggestionsS2c<'a> {
    pub id: VarInt,
    pub start: VarInt,
    pub length: VarInt,
    pub matches: Vec<CommandSuggestionsMatch<'a>>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct CommandSuggestionsMatch<'a> {
    pub suggested_match: &'a str,
    pub tooltip: Option<Cow<'a, Text>>,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PROFILELESS_CHAT_MESSAGE_S2C)]
pub struct ProfilelessChatMessageS2c<'a> {
    pub message: Cow<'a, Text>,
    pub chat_type: VarInt,
    pub chat_type_name: Cow<'a, Text>,
    pub target_name: Option<Cow<'a, Text>>,
}
