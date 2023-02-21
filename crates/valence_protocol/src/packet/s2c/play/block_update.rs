use crate::block_pos::BlockPos;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x09]
pub struct BlockUpdateS2c {
    pub position: BlockPos,
    pub block_id: VarInt,
}
