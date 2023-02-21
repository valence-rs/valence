use std::borrow::Cow;

use crate::text::Text;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x34]
pub struct DeathMessageS2c<'a> {
    pub player_id: VarInt,
    /// Killer's entity ID, -1 if no killer
    pub entity_id: i32,
    pub message: Cow<'a, Text>,
}
