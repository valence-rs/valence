use std::borrow::Cow;

use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};
use crate::text::Text;

#[derive(Clone, Debug, Encode, Decode)]
pub struct DeathMessageS2c<'a> {
    pub player_id: VarInt,
    /// Killer's entity ID, -1 if no killer
    pub entity_id: i32,
    pub message: Cow<'a, Text>,
}
