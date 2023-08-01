use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_ANIMATION_S2C)]
pub struct EntityAnimationS2c {
    pub entity_id: VarInt,
    pub animation: u8,
}
