use valence_math::DVec3;

use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct VehicleMoveC2s {
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
}
