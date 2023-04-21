use std::borrow::Cow;

use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct EntitiesDestroyS2c<'a> {
    pub entity_ids: Cow<'a, [VarInt]>,
}
