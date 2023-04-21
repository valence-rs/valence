use crate::block_pos::BlockPos;
use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct WorldEventS2c {
    pub event: i32,
    pub location: BlockPos,
    pub data: i32,
    pub disable_relative_volume: bool,
}
