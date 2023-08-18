use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct NbtQueryResponseS2c {
    pub transaction_id: VarInt,
    pub nbt: Compound,
}
