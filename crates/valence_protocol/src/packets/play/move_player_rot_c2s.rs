use crate::{movement_flags::MovementFlags, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct MovePlayerRotC2s {
    pub yaw: f32,
    pub pitch: f32,
    pub flags: MovementFlags,
}
