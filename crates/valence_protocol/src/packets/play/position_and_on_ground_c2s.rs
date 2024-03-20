use valence_math::DVec3;

use crate::{PacketSide, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(name = "POSITION_AND_ON_GROUND", side = PacketSide::Serverbound)]
pub struct PositionAndOnGroundC2s {
    pub position: DVec3,
    pub on_ground: bool,
}
