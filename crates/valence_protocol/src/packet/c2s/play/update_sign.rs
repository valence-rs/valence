use crate::block_pos::BlockPos;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x2e]
pub struct UpdateSignC2s<'a> {
    pub position: BlockPos,
    pub lines: [&'a str; 4],
}
