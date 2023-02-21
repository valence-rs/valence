use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, PartialEq, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x38]
pub struct PlayerPositionLookS2c {
    pub position: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub flags: Flags,
    pub teleport_id: VarInt,
    pub dismount_vehicle: bool,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct Flags {
    pub x: bool,
    pub y: bool,
    pub z: bool,
    pub y_rot: bool,
    pub x_rot: bool,
    #[bits(3)]
    _pad: u8,
}
