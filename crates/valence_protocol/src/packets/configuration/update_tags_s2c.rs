use std::borrow::Cow;

use crate::packets::play::update_tags_s2c::RegistryMap;
use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct UpdateTagsS2c<'a> {
    pub groups: Cow<'a, RegistryMap>,
}
