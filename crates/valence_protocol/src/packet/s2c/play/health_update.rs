use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x53]
pub struct HealthUpdateS2c {
    pub health: f32,
    pub food: VarInt,
    pub food_saturation: f32,
}
