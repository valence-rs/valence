use crate::block_pos::BlockPos;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct UpdateSignC2s<'a> {
    pub position: BlockPos,
    pub lines: [&'a str; 4],
}
