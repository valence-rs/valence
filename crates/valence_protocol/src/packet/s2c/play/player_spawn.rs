use uuid::Uuid;

use crate::byte_angle::ByteAngle;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlayerSpawnS2c {
    pub entity_id: VarInt,
    pub player_uuid: Uuid,
    pub position: [f64; 3],
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
}
