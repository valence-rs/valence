use crate::{Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct EntityAnimationS2c {
    pub entity_id: VarInt,
    pub animation: u8,
}
