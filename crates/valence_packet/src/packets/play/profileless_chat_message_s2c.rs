use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PROFILELESS_CHAT_MESSAGE_S2C)]
pub struct ProfilelessChatMessageS2c<'a> {
    pub message: Cow<'a, Text>,
    pub chat_type: VarInt,
    pub chat_type_name: Cow<'a, Text>,
    pub target_name: Option<Cow<'a, Text>>,
}
