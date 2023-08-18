use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct BlockEventS2c {
    pub position: BlockPos,
    pub action_id: u8,
    pub action_parameter: u8,
    pub block_type: BlockKind,
}
