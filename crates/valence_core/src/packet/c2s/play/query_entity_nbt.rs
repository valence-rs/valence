use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct QueryEntityNbtC2s {
    pub transaction_id: VarInt,
    pub entity_id: VarInt,
}
