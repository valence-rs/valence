use crate::{Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct SetChunkCacheRadiusS2c {
    pub view_distance: VarInt,
}
