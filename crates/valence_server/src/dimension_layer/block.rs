use valence_nbt::Compound;
pub use valence_protocol::BlockState;

/// Represents a complete block, which is a pair of block state and optional NBT
/// data for the block entity.
#[derive(Clone, PartialEq, Default, Debug)]
pub struct Block {
    pub state: BlockState,
    pub nbt: Option<Compound>,
}

impl Block {
    pub const fn new(state: BlockState, nbt: Option<Compound>) -> Self {
        Self { state, nbt }
    }
}

impl From<BlockState> for Block {
    fn from(state: BlockState) -> Self {
        Self { state, nbt: None }
    }
}

/// Like [`Block`] but immutably referenced.
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct BlockRef<'a> {
    pub state: BlockState,
    pub nbt: Option<&'a Compound>,
}

impl<'a> BlockRef<'a> {
    pub const fn new(state: BlockState, nbt: Option<&'a Compound>) -> Self {
        Self { state, nbt }
    }
}

impl From<BlockRef<'_>> for Block {
    fn from(value: BlockRef<'_>) -> Self {
        Self {
            state: value.state,
            nbt: value.nbt.cloned(),
        }
    }
}

impl From<BlockState> for BlockRef<'_> {
    fn from(state: BlockState) -> Self {
        Self { state, nbt: None }
    }
}

impl<'a> From<&'a Block> for BlockRef<'a> {
    fn from(value: &'a Block) -> Self {
        Self {
            state: value.state,
            nbt: value.nbt.as_ref(),
        }
    }
}
