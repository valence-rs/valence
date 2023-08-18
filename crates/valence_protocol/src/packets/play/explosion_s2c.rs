use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ExplosionS2c<'a> {
    pub pos: DVec3,
    pub radius: f32,
    pub affected_blocks: Cow<'a, [BlockPos]>,
    pub player_velocity: Velocity,
}
