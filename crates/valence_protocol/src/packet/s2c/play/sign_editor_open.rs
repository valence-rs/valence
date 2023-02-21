use crate::block_pos::BlockPos;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x2d]
pub struct SignEditorOpen {
    pub location: BlockPos,
}
