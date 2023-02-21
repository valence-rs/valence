use uuid::Uuid;

use crate::byte_angle::ByteAngle;
use crate::var_int::VarInt;
use crate::{Decode, Encode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x00]
pub struct EntitySpawnS2c {
    pub entity_id: VarInt,
    pub object_uuid: Uuid,
    // TODO: EntityKind type?
    pub kind: VarInt,
    pub position: [f64; 3],
    pub pitch: ByteAngle,
    pub yaw: ByteAngle,
    pub head_yaw: ByteAngle,
    pub data: VarInt,
    pub velocity: [i16; 3],
}
