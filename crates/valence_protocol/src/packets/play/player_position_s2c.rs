use bitfield_struct::bitfield;
use valence_math::DVec3;

use crate::{Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode, Packet)]
pub struct PlayerPositionS2c {
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub flags: TeleportRelativeFlags,
    pub teleport_id: VarInt,
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct TeleportRelativeFlags {
    pub x: bool,
    pub y: bool,
    pub z: bool,
    pub y_rot: bool,
    pub x_rot: bool,
    pub x_vel: bool,
    pub y_vel: bool,
    pub z_vel: bool,
    pub rot_vel: bool,
    #[bits(23)]
    _pad: u32,
}
