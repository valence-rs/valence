use valence_math::DVec3;

use crate::{ByteAngle, Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct EntityPositionS2c {
    pub entity_id: VarInt,
    pub position: DVec3,
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
    pub on_ground: bool,
}
