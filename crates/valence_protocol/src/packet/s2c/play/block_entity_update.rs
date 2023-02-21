use std::borrow::Cow;

use valence_nbt::Compound;

use crate::block::BlockEntityKind;
use crate::block_pos::BlockPos;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct BlockEntityUpdateS2c<'a> {
    pub position: BlockPos,
    pub kind: BlockEntityKind,
    pub data: Cow<'a, Compound>,
}
