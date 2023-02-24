use valence_nbt::Compound;

use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct NbtQueryResponseS2c {
    pub transaction_id: VarInt,
    pub nbt: Compound,
}
