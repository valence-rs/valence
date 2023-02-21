use valence_nbt::Compound;

use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x62]
pub struct NbtQueryResponseS2c {
    pub transaction_id: VarInt,
    pub nbt: Compound,
}
