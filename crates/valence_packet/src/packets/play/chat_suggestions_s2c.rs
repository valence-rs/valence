use super::*;

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
