use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::NBT_QUERY_RESPONSE_S2C)]
pub struct NbtQueryResponseS2c {
    pub transaction_id: VarInt,
    pub nbt: Compound,
}
