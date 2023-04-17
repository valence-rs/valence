use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct UpdateBeaconC2s {
    pub primary_effect: Option<VarInt>,
    pub secondary_effect: Option<VarInt>,
}
