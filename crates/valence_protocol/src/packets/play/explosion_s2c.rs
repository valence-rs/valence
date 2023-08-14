use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::EXPLOSION_S2C)]
pub struct ExplosionS2c<'a> {
    pub pos: DVec3,
    pub radius: f32,
    pub affected_blocks: Cow<'a, [BlockPos]>,
    pub player_velocity: Velocity,
}
