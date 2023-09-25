use uuid::Uuid;
use valence_math::DVec3;

use crate::{ByteAngle, Decode, Encode, Packet, VarInt, Velocity};

/// Sent by the server when a vehicle or other non-living entity is created.
///
/// wiki : [Spawn Entity](https://wiki.vg/Protocol#Spawn_Experience_Orb)
#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct EntitySpawnS2c {
    pub entity_id: VarInt,
    pub object_uuid: Uuid,
    pub kind: VarInt, // TODO: EntityKind in valence_generated?
    pub position: DVec3,
    pub pitch: ByteAngle,
    pub yaw: ByteAngle,
    pub head_yaw: ByteAngle,
    pub data: VarInt,
    pub velocity: Velocity,
}
