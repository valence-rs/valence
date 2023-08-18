use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct QueryBlockNbtC2s {
    pub transaction_id: VarInt,
    pub position: BlockPos,
}
