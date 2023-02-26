use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ChunkLoadDistanceS2c {
    pub view_distance: VarInt,
}
