use crate::block_pos::BlockPos;
use crate::var_int::VarInt;
use crate::{Encode, Decode};
use crate::types::BlockFace;

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlayerActionC2s {
    pub status: DiggingStatus,
    pub position: BlockPos,
    pub face: BlockFace,
    pub sequence: VarInt,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum DiggingStatus {
    StartDestroyBlock,
    AbortDestroyBlock,
    StopDestroyBlock,
    DropAllItems,
    DropItem,
    ReleaseUseItem,
    SwapItemWithOffhand,
}
