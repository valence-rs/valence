use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::QUERY_ENTITY_NBT_C2S)]
pub struct QueryEntityNbtC2s {
    pub transaction_id: VarInt,
    pub entity_id: VarInt,
}
