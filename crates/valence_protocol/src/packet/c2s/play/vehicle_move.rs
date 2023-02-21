use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x17]
pub struct VehicleMoveC2s {
    pub position: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
}
