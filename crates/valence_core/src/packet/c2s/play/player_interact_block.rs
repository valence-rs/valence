use crate::block_pos::BlockPos;
use crate::direction::Direction;
use crate::hand::Hand;
use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlayerInteractBlockC2s {
    pub hand: Hand,
    pub position: BlockPos,
    pub face: Direction,
    pub cursor_pos: [f32; 3],
    pub head_inside_block: bool,
    pub sequence: VarInt,
}
