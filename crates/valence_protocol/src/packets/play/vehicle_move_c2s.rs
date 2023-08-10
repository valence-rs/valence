use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::VEHICLE_MOVE_C2S)]
pub struct VehicleMoveC2s {
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
}
