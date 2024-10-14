use crate::{Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct SetExperienceS2c {
    pub bar: f32,
    pub level: VarInt,
    pub total_xp: VarInt,
}
