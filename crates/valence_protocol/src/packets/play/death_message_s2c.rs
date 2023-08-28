use std::borrow::Cow;

use valence_text::Text;

use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct DeathMessageS2c<'a> {
    pub player_id: VarInt,
    pub message: Cow<'a, Text>,
}
