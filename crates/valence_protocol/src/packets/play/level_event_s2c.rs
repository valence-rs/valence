use crate::{BlockPos, Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct LevelEventS2c {
    pub event: i32,
    pub location: BlockPos,
    pub data: i32,
    pub disable_relative_volume: bool,
}
