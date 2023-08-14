use super::*;

/// Sent by the server when a vehicle or other non-living entity is created.
///
/// wiki : [Spawn Entity](https://wiki.vg/Protocol#Spawn_Experience_Orb)
#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_SPAWN_S2C)]
pub struct EntitySpawnS2c {
    pub entity_id: VarInt,
    pub object_uuid: Uuid,
    pub kind: VarInt,
    pub position: DVec3,
    pub pitch: ByteAngle,
    pub yaw: ByteAngle,
    pub head_yaw: ByteAngle,
    pub data: VarInt,
    pub velocity: [i16; 3],
}
