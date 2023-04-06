use std::borrow::Cow;

use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct ChunkBiomeDataS2c<'a> {
    pub chunks: Cow<'a, [ChunkBiome<'a>]>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ChunkBiome<'a> {
    pub chunk_x: i32,
    pub chunk_z: i32,
    /// Chunk data structure, with sections containing only the `Biomes` field.
    pub data: &'a [u8],
}
