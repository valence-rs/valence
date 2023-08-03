use valence_core::protocol::array::LengthPrefixedArray;

use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LIGHT_UPDATE_S2C)]
pub struct LightUpdateS2c {
    pub chunk_x: VarInt,
    pub chunk_z: VarInt,
    pub sky_light_mask: Vec<u64>,
    pub block_light_mask: Vec<u64>,
    pub empty_sky_light_mask: Vec<u64>,
    pub empty_block_light_mask: Vec<u64>,
    pub sky_light_arrays: Vec<LengthPrefixedArray<u8, 2048>>,
    pub block_light_arrays: Vec<LengthPrefixedArray<u8, 2048>>,
}
