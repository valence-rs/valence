use super::*;

#[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::REMOVE_ENTITY_STATUS_EFFECT_S2C)]
pub struct RemoveEntityStatusEffectS2c {
    pub entity_id: VarInt,
    pub effect_id: VarInt,
}
