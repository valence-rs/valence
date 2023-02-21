use std::borrow::Cow;

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x14]
pub struct ChatSuggestionsS2c<'a> {
    pub action: Action,
    pub entries: Cow<'a, [&'a str]>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum Action {
    Add,
    Remove,
    Set,
}
