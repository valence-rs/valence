use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UNLOAD_CHUNK_S2C)]
pub struct UnloadChunkS2c {
    pub pos: ChunkPos,
}
