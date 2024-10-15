use valence_nbt::Compound;

use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct TagQueryS2c {
    pub transaction_id: VarInt,
    pub nbt: Compound,
}
