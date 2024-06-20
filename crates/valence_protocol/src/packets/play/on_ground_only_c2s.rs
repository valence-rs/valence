use crate::{PacketSide, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(name = "ON_GROUND_ONLY", side = PacketSide::Serverbound)]
pub struct OnGroundOnlyC2s {
    pub on_ground: bool,
}
