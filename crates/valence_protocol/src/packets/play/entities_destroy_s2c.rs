use std::borrow::Cow;

use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
pub struct EntitiesDestroyS2c<'a> {
    pub entity_ids: Cow<'a, [VarInt]>,
}
