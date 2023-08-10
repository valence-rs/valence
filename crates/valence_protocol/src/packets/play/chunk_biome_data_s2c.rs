use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CHUNK_BIOME_DATA_S2C)]
pub struct ChunkBiomeDataS2c<'a> {
    pub chunks: Cow<'a, [ChunkBiome<'a>]>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ChunkBiome<'a> {
    pub pos: ChunkPos,
    /// Chunk data structure, with sections containing only the `Biomes` field.
    pub data: &'a [u8],
}
