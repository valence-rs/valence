use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::EXPERIENCE_BAR_UPDATE_S2C)]
pub struct ExperienceBarUpdateS2c {
    pub bar: f32,
    pub level: VarInt,
    pub total_xp: VarInt,
}
