use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::VEHICLE_MOVE_S2C)]
pub struct VehicleMoveS2c {
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
}
