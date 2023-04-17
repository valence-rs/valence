use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ChunkLoadDistanceS2c {
    pub view_distance: VarInt,
}
