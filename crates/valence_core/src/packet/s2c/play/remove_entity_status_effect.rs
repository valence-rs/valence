use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct RemoveEntityStatusEffectS2c {
    pub entity_id: VarInt,
    pub effect_id: VarInt,
}
