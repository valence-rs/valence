use bitfield_struct::bitfield;

use crate::{BlockPos, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct SetCommandBlockC2s<'a> {
    pub position: BlockPos,
    pub command: &'a str,
    pub mode: UpdateCommandBlockMode,
    pub flags: UpdateCommandBlockFlags,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum UpdateCommandBlockMode {
    Sequence,
    Auto,
    Redstone,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct UpdateCommandBlockFlags {
    pub track_output: bool,
    pub conditional: bool,
    pub automatic: bool,
    #[bits(5)]
    _pad: u8,
}
