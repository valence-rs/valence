use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, PartialEq, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x3b]
pub struct RemoveEntityStatusEffectS2c {
    pub entity_id: VarInt,
    pub effect_id: VarInt,
}
