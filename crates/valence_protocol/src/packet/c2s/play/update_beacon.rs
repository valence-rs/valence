use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct UpdateBeaconC2s {
    // TODO: extract effect IDs?
    pub primary_effect: Option<VarInt>,
    pub secondary_effect: Option<VarInt>,
}
