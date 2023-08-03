use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::GAME_MESSAGE_S2C)]
pub struct GameMessageS2c<'a> {
    pub chat: Cow<'a, Text>,
    /// Whether the message is in the actionbar or the chat.
    pub overlay: bool,
}
