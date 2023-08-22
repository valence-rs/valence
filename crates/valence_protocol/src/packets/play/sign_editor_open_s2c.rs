use crate::{BlockPos, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct SignEditorOpenS2c {
    pub location: BlockPos,
}
