use bitfield_struct::bitfield;

use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlayerInputC2s {
    pub sideways: f32,
    pub forward: f32,
    pub flags: Flags,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct Flags {
    pub jump: bool,
    pub unmount: bool,
    #[bits(6)]
    _pad: u8,
}
