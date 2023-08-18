use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct GameMessageS2c<'a> {
    pub chat: Cow<'a, Text>,
    /// Whether the message is in the actionbar or the chat.
    pub overlay: bool,
}
