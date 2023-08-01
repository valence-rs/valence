use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::BLOCK_BREAKING_PROGRESS_S2C)]
pub struct BlockBreakingProgressS2c {
    pub entity_id: VarInt,
    pub position: BlockPos,
    pub destroy_stage: u8,
}
