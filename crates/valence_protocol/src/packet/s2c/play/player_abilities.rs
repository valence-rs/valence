use bitfield_struct::bitfield;

use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct PlayerAbilitiesS2c {
    pub flags: PlayerAbilitiesFlags,
    pub flying_speed: f32,
    pub fov_modifier: f32,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct PlayerAbilitiesFlags {
    pub invulnerable: bool,
    pub flying: bool,
    pub allow_flying: bool,
    pub instant_break: bool,
    #[bits(4)]
    _pad: u8,
}
