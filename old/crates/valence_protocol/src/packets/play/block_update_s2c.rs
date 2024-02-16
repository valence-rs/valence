use crate::{BlockPos, BlockState, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct BlockUpdateS2c {
    pub position: BlockPos,
    pub block_id: BlockState,
}
