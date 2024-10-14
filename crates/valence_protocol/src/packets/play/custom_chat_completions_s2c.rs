use std::borrow::Cow;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct CustomChatCompletionsS2c<'a> {
    pub action: ChatSuggestionsAction,
    pub entries: Cow<'a, [&'a str]>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum ChatSuggestionsAction {
    Add,
    Remove,
    Set,
}
