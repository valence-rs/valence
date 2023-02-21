use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x4a]
pub struct ChunkRenderDistanceCenterS2c {
    pub chunk_x: VarInt,
    pub chunk_z: VarInt,
}
