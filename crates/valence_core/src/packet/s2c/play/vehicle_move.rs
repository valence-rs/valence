use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct VehicleMoveS2c {
    pub position: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
}
