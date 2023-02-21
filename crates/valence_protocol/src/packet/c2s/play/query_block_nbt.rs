use crate::block_pos::BlockPos;
use crate::var_int::VarInt;

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x01]
pub struct QueryBlockNbtC2s {
    pub transaction_id: VarInt,
    pub position: BlockPos,
}
