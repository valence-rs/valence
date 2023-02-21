use crate::block_pos::BlockPos;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x06]
pub struct BlockBreakingProgressS2c {
    pub entity_id: VarInt,
    pub position: BlockPos,
    pub destroy_stage: u8,
}
