use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct WorldBorderInitializeS2c {
    pub x: f64,
    pub z: f64,
    pub old_diameter: f64,
    pub new_diameter: f64,
    pub duration_millis: VarLong,
    pub portal_teleport_boundary: VarInt,
    pub warning_blocks: VarInt,
    pub warning_time: VarInt,
}
