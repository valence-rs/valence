use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct ChunkLoadDistanceS2c {
    pub view_distance: VarInt,
}
