use crate::{PacketSide, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(name = "LOOK_AND_ON_GROUND", side = PacketSide::Serverbound)]
pub struct LookAndOnGroundC2s {
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}
