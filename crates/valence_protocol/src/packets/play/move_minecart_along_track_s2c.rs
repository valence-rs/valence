use crate::{ByteAngle, Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct MoveMinecartAlongTrackS2c {
    pub entity_id: VarInt,
    pub steps: Vec<Step>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct Step {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub velocity_x: f64,
    pub velocity_y: f64,
    pub velocity_z: f64,
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
    pub weight: f32,
}
