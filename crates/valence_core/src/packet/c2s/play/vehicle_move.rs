use glam::DVec3;

use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct VehicleMoveC2s {
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
}
