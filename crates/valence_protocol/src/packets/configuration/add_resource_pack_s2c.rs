use std::borrow::Cow;
use uuid::Uuid;
use valence_text::TextContent;
use crate::{Bounded, Decode, Encode, Packet, PacketState};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
pub struct AddResourcePackS2c<'a> {
    pub uuid: Uuid,
    pub url: Bounded<Cow<'a, str>, 32767>,
    pub hash: Bounded<Cow<'a, str>, 40>,
    pub prompt_message: Option<TextContent>,
}
