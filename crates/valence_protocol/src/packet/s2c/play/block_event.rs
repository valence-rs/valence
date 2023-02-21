use crate::block_pos::BlockPos;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x08]
pub struct BlockEventS2c {
    pub position: BlockPos,
    pub action_id: u8,
    pub action_parameter: u8,
    pub block_type: VarInt,
}
