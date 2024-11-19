use std::borrow::Cow;

use crate::{Decode, Encode, Ident, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct CooldownS2c<'a> {
    pub cooldown_group: Ident<Cow<'a, str>>,
    pub cooldown_ticks: VarInt,
}
