use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_SPAWN_POSITION_S2C)]
pub struct PlayerSpawnPositionS2c {
    pub position: BlockPos,
    pub angle: f32,
}
