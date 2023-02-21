use std::borrow::Cow;
use crate::{Encode, Decode};

use valence_nbt::Compound;

use crate::array::LengthPrefixedArray;
use crate::block::BlockEntityKind;

#[derive(Clone, Debug, Encode, Decode)]
pub struct ChunkDataS2c<'a> {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub heightmaps: Cow<'a, Compound>,
    pub blocks_and_biomes: &'a [u8],
    pub block_entities: Cow<'a, [ChunkDataBlockEntity<'a>]>,
    pub trust_edges: bool,
    pub sky_light_mask: Cow<'a, [u64]>,
    pub block_light_mask: Cow<'a, [u64]>,
    pub empty_sky_light_mask: Cow<'a, [u64]>,
    pub empty_block_light_mask: Cow<'a, [u64]>,
    pub sky_light_arrays: Cow<'a, [LengthPrefixedArray<u8, 2048>]>,
    pub block_light_arrays: Cow<'a, [LengthPrefixedArray<u8, 2048>]>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct ChunkDataBlockEntity<'a> {
    pub packed_xz: i8,
    pub y: i16,
    pub kind: BlockEntityKind,
    pub data: Cow<'a, Compound>,
}
