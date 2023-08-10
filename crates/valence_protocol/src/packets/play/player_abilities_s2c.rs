use bevy_ecs::prelude::Component;

use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_ABILITIES_S2C)]
pub struct PlayerAbilitiesS2c {
    pub flags: PlayerAbilitiesFlags,
    pub flying_speed: f32,
    pub fov_modifier: f32,
}

/// [`Component`] that stores the player's abilities flags.
#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode, Component, Default)]
pub struct PlayerAbilitiesFlags {
    pub invulnerable: bool,
    pub flying: bool,
    pub allow_flying: bool,
    pub instant_break: bool,
    #[bits(4)]
    _pad: u8,
}
