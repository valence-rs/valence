use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x2a]
pub struct VehicleMoveS2c {
    pub position: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
}
