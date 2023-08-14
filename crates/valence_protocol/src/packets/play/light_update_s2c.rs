use super::*;
use crate::array::LengthPrefixedArray;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LIGHT_UPDATE_S2C)]
pub struct LightUpdateS2c<'a> {
    pub chunk_x: VarInt,
    pub chunk_z: VarInt,
    pub sky_light_mask: Cow<'a, [u64]>,
    pub block_light_mask: Cow<'a, [u64]>,
    pub empty_sky_light_mask: Cow<'a, [u64]>,
    pub empty_block_light_mask: Cow<'a, [u64]>,
    pub sky_light_arrays: Cow<'a, [LengthPrefixedArray<u8, 2048>]>,
    pub block_light_arrays: Cow<'a, [LengthPrefixedArray<u8, 2048>]>,
}
