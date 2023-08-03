use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::HEALTH_UPDATE_S2C)]
pub struct HealthUpdateS2c {
    pub health: f32,
    pub food: VarInt,
    pub food_saturation: f32,
}
