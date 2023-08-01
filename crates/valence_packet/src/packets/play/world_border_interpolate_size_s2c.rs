use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::WORLD_BORDER_INTERPOLATE_SIZE_S2C)]
pub struct WorldBorderInterpolateSizeS2c {
    pub old_diameter: f64,
    pub new_diameter: f64,
    pub speed: VarLong,
}
