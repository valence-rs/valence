use valence_math::Vec3;

use crate::{BlockPos, Decode, Direction, Encode, Hand, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct UseItemOnC2s {
    pub hand: Hand,
    pub position: BlockPos,
    pub face: Direction,
    pub cursor_pos: Vec3,
    pub head_inside_block: bool,
    pub sequence: VarInt,
}
