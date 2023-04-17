use crate::chunk_pos::ChunkPos;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct UnloadChunkS2c {
    pub pos: ChunkPos,
}
