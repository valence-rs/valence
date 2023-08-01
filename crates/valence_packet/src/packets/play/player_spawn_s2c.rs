use super::*;

/// This packet is sent by the server when a player comes into visible range,
/// not when a player joins.
///
/// This packet must be sent after the Player Info Update packet that adds the
/// player data for the client to use when spawning a player. If the Player Info
/// for the player spawned by this packet is not present when this packet
/// arrives, Notchian clients will not spawn the player entity. The Player Info
/// packet includes skin/cape data.
///
/// Servers can, however, safely spawn player entities for players not in
/// visible range. The client appears to handle it correctly.
///
/// wiki : [Spawn Player](https://wiki.vg/Protocol#Spawn_Player)
#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_SPAWN_S2C)]
pub struct PlayerSpawnS2c {
    /// A unique integer ID mostly used in the protocol to identify the player.
    pub entity_id: VarInt,
    pub player_uuid: Uuid,
    pub position: DVec3,
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
}
