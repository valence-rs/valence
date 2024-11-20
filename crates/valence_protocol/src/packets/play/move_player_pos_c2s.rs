use valence_math::DVec3;

use crate::{movement_flags::MovementFlags, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct MovePlayerPosC2s {
    pub position: DVec3,
    pub flags: MovementFlags,
}
