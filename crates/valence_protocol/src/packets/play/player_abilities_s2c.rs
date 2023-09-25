use bevy_ecs::prelude::Component;
use bitfield_struct::bitfield;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct PlayerAbilitiesS2c {
    pub flags: PlayerAbilitiesFlags,
    pub flying_speed: f32,
    pub fov_modifier: f32,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode, Component)]
pub struct PlayerAbilitiesFlags {
    pub invulnerable: bool,
    pub flying: bool,
    pub allow_flying: bool,
    pub instant_break: bool,
    #[bits(4)]
    _pad: u8,
}
