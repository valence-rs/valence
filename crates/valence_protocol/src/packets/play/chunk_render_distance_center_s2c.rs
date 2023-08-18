use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct ChunkRenderDistanceCenterS2c {
    pub chunk_x: VarInt,
    pub chunk_z: VarInt,
}
