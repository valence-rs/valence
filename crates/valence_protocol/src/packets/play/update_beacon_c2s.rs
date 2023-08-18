use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct UpdateBeaconC2s {
    pub primary_effect: Option<VarInt>,
    pub secondary_effect: Option<VarInt>,
}
