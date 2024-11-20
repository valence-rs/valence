use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct PlayerRotationS2c {
    pub yaw: f32,
    pub pitch: f32,
}
