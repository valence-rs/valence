use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CHUNK_RENDER_DISTANCE_CENTER_S2C)]
pub struct ChunkRenderDistanceCenterS2c {
    pub chunk_x: VarInt,
    pub chunk_z: VarInt,
}
