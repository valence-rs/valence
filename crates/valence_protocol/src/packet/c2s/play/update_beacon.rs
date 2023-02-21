use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x27]
pub struct UpdateBeaconC2s {
    // TODO: extract effect IDs?
    pub primary_effect: Option<VarInt>,
    pub secondary_effect: Option<VarInt>,
}
