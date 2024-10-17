use valence_math::DVec3;

use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct MovePlayerPosC2s {
    pub position: DVec3,
    pub on_ground: bool,
}
