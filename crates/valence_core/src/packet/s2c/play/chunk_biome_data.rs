use std::borrow::Cow;

use crate::chunk_pos::ChunkPos;
use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct ChunkBiomeDataS2c<'a> {
    pub chunks: Cow<'a, [ChunkBiome<'a>]>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ChunkBiome<'a> {
    pub pos: ChunkPos,
    /// Chunk data structure, with sections containing only the `Biomes` field.
    pub data: &'a [u8],
}
