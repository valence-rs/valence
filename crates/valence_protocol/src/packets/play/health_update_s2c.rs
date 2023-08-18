use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct HealthUpdateS2c {
    pub health: f32,
    pub food: VarInt,
    pub food_saturation: f32,
}
