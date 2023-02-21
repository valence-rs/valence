use crate::block_pos::BlockPos;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x21]
pub struct WorldEventS2c {
    pub event: i32,
    pub location: BlockPos,
    pub data: i32,
    pub disable_relative_volume: bool,
}
