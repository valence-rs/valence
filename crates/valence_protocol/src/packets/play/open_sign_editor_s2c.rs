use crate::{BlockPos, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct OpenSignEditorS2c {
    pub location: BlockPos,
    pub is_front_text: bool,
}
