use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x19]
pub struct PickFromInventoryC2s {
    pub slot_to_use: VarInt,
}
