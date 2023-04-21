use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PickFromInventoryC2s {
    pub slot_to_use: VarInt,
}
