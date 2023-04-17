use glam::DVec3;
use uuid::Uuid;

use crate::packet::byte_angle::ByteAngle;
use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlayerSpawnS2c {
    pub entity_id: VarInt,
    pub player_uuid: Uuid,
    pub position: DVec3,
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
}
