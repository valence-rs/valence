use std::borrow::Cow;

use valence_generated::block::BlockEntityKind;
use valence_nbt::Compound;

use crate::{BlockPos, Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct BlockEntityUpdateS2c<'a> {
    pub position: BlockPos,
    pub kind: BlockEntityKind,
    pub data: Cow<'a, Compound>,
}
