use bitfield_struct::bitfield;

use crate::{Decode, Encode};

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct MovementFlags {
    pub on_ground: bool,
    pub pushing_against_wall: bool,
    #[bits(6)]
    _padding: u8,
}
