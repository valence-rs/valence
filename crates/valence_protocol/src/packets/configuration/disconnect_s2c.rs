use std::borrow::Cow;

use valence_text::Text;

use crate::{Decode, Encode, Packet, PacketState};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
pub struct DisconnectS2c<'a> {
    pub reason: Cow<'a, Text>,
}
