use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::QUERY_BLOCK_NBT_C2S)]
pub struct QueryBlockNbtC2s {
    pub transaction_id: VarInt,
    pub position: BlockPos,
}
