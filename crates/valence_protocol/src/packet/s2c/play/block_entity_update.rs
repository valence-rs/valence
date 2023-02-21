use std::borrow::Cow;

use valence_nbt::Compound;

use crate::block_pos::BlockPos;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x07]
pub struct BlockEntityUpdateS2c<'a> {
    pub position: BlockPos,
    pub kind: BlockEntityKind,
    pub data: Cow<'a, Compound>,
}
