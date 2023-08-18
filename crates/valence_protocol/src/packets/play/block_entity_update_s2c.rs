use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct BlockEntityUpdateS2c<'a> {
    pub position: BlockPos,
    pub kind: BlockEntityKind,
    pub data: Cow<'a, Compound>,
}
