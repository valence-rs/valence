use crate::{packet_id, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ON_GROUND_ONLY)]
pub struct OnGroundOnlyC2s {
    pub on_ground: bool,
}
