use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x52]
pub struct ExperienceBarUpdateS2c {
    pub bar: f32,
    pub level: VarInt,
    pub total_xp: VarInt,
}
