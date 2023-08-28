use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
pub struct RemoveEntityStatusEffectS2c {
    pub entity_id: VarInt,
    pub effect_id: VarInt,
}
