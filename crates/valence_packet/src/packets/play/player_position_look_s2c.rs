use super::*;

#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_POSITION_LOOK_S2C)]
pub struct PlayerPositionLookS2c {
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub flags: PlayerPositionLookFlags,
    pub teleport_id: VarInt,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct PlayerPositionLookFlags {
    pub x: bool,
    pub y: bool,
    pub z: bool,
    pub y_rot: bool,
    pub x_rot: bool,
    #[bits(3)]
    _pad: u8,
}
