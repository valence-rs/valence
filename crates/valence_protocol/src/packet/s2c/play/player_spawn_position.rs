use crate::block_pos::BlockPos;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x4c]
pub struct PlayerSpawnPositionS2c {
    pub position: BlockPos,
    pub angle: f32,
}
