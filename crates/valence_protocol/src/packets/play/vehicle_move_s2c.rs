use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct VehicleMoveS2c {
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
}
