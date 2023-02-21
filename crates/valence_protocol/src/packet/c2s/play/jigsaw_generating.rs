use crate::block_pos::BlockPos;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x10]
pub struct JigsawGeneratingC2s {
    pub position: BlockPos,
    pub levels: VarInt,
    pub keep_jigsaws: bool,
}
