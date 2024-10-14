use std::borrow::Cow;

use crate::array::FixedArray;
use crate::{Decode, Encode, Packet, VarInt};
// TODO: fix this
#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct LightUpdateS2c<'a> {
    pub chunk_x: VarInt,
    pub chunk_z: VarInt,
    pub sky_light_mask: Cow<'a, [u64]>,
    pub block_light_mask: Cow<'a, [u64]>,
    pub empty_sky_light_mask: Cow<'a, [u64]>,
    pub empty_block_light_mask: Cow<'a, [u64]>,
    pub sky_light_arrays: Cow<'a, [FixedArray<u8, 2048>]>,
    pub block_light_arrays: Cow<'a, [FixedArray<u8, 2048>]>,
}
