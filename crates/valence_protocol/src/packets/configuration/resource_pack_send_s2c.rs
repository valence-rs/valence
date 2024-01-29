use std::borrow::Cow;

use uuid::Uuid;
use valence_text::Text;

use crate::{Bounded, Decode, Encode, Packet, PacketState};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
pub struct ResourcePackSendS2c<'a> {
    pub uuid: Uuid,
    pub url: &'a str,
    pub hash: Bounded<&'a str, 40>,
    pub forced: bool,
    pub prompt_message: Option<Cow<'a, Text>>,
}
