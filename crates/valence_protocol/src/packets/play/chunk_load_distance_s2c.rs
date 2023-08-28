use crate::{Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct ChunkLoadDistanceS2c {
    pub view_distance: VarInt,
}
