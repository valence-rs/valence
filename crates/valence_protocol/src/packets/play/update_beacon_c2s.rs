use crate::{Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct UpdateBeaconC2s {
    pub primary_effect: Option<VarInt>,
    pub secondary_effect: Option<VarInt>,
}
