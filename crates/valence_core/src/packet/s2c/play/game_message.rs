use std::borrow::Cow;

use crate::packet::{Decode, Encode};
use crate::text::Text;

#[derive(Clone, Debug, Encode, Decode)]
pub struct GameMessageS2c<'a> {
    pub chat: Cow<'a, Text>,
    /// Whether the message is in the actionbar or the chat.
    pub overlay: bool,
}
