use crate::block_pos::BlockPos;
use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct BlockUpdateS2c {
    pub position: BlockPos,
    pub block_id: VarInt,
}
