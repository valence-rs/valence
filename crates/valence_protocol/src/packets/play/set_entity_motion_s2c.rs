use crate::{Decode, Encode, Packet, VarInt, Velocity};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct SetEntityMotionS2c {
    pub entity_id: VarInt,
    pub velocity: Velocity,
}
