use bitfield_struct::bitfield;

use crate::block_pos::BlockPos;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct UpdateCommandBlockC2s<'a> {
    pub position: BlockPos,
    pub command: &'a str,
    pub mode: Mode,
    pub flags: Flags,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum Mode {
    Sequence,
    Auto,
    Redstone,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct Flags {
    pub track_output: bool,
    pub conditional: bool,
    pub automatic: bool,
    #[bits(5)]
    _pad: u8,
}
