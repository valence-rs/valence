use std::borrow::Cow;

use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, PartialEq, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x3a]
pub struct EntitiesDestroyS2c<'a> {
    pub entity_ids: Cow<'a, [VarInt]>,
}
