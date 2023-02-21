use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct UnloadChunkS2c {
    pub chunk_x: i32,
    pub chunk_z: i32,
}
