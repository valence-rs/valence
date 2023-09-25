use valence_math::DVec3;

use crate::{packet_id, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::FULL)]
pub struct FullC2s {
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}
