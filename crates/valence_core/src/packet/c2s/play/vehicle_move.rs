use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct VehicleMoveC2s {
    pub position: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
}
