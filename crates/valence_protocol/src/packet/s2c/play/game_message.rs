use std::borrow::Cow;

use crate::text::Text;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x60]
pub struct GameMessageS2c<'a> {
    pub chat: Cow<'a, Text>,
    /// Whether the message is in the actionbar or the chat.
    pub overlay: bool,
}
