use std::borrow::Cow;

use crate::packets::play::update_tags_s2c::RegistryMap;
use crate::{Decode, Encode, Packet, PacketState};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
pub struct UpdateTagsS2c<'a> {
    pub groups: Cow<'a, RegistryMap>,
}
