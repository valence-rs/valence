use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct SignEditorOpenS2c {
    pub location: BlockPos,
}
