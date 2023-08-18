use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct QueryEntityNbtC2s {
    pub transaction_id: VarInt,
    pub entity_id: VarInt,
}
