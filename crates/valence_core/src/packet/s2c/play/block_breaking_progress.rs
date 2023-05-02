use crate::block_pos::BlockPos;
use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct BlockBreakingProgressS2c {
    pub entity_id: VarInt,
    pub position: BlockPos,
    pub destroy_stage: u8,
}
