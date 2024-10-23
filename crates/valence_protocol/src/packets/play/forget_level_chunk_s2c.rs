use crate::{ChunkPos, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct ForgetLevelChunkS2c {
    pub pos: ChunkPos,
}
