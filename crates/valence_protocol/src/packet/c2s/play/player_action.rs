use crate::block::BlockFace;
use crate::block_pos::BlockPos;
use crate::var_int::VarInt;

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x1c]
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
