use std::borrow::Cow;

use valence_core::block_pos::BlockPos;
use valence_core::chunk_pos::ChunkPos;
use valence_core::ident::Ident;
use valence_core::protocol::array::LengthPrefixedArray;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::var_long::VarLong;
use valence_core::protocol::{packet_id, Decode, Encode, Packet};
use valence_nbt::Compound;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::WORLD_EVENT_S2C)]
pub struct WorldEventS2c {
    pub event: i32,
    pub location: BlockPos,
    pub data: i32,
    pub disable_relative_volume: bool,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CHUNK_BIOME_DATA_S2C)]
pub struct ChunkBiomeDataS2c<'a> {
    pub chunks: Cow<'a, [ChunkBiome<'a>]>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ChunkBiome<'a> {
    pub pos: ChunkPos,
    /// Chunk data structure, with sections containing only the `Biomes` field.
    pub data: &'a [u8],
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CHUNK_DATA_S2C)]
pub struct ChunkDataS2c<'a> {
    pub pos: ChunkPos,
    pub heightmaps: Cow<'a, Compound>,
    pub blocks_and_biomes: &'a [u8],
    pub block_entities: Cow<'a, [ChunkDataBlockEntity<'a>]>,
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
    pub kind: VarInt,
    pub data: Cow<'a, Compound>,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CHUNK_DELTA_UPDATE_S2C)]
pub struct ChunkDeltaUpdateS2c<'a> {
    pub chunk_section_position: i64,
    pub blocks: Cow<'a, [VarLong]>,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CHUNK_LOAD_DISTANCE_S2C)]
pub struct ChunkLoadDistanceS2c {
    pub view_distance: VarInt,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UNLOAD_CHUNK_S2C)]
pub struct UnloadChunkS2c {
    pub pos: ChunkPos,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CHUNK_RENDER_DISTANCE_CENTER_S2C)]
pub struct ChunkRenderDistanceCenterS2c {
    pub chunk_x: VarInt,
    pub chunk_z: VarInt,
}

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

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::BLOCK_BREAKING_PROGRESS_S2C)]
pub struct BlockBreakingProgressS2c {
    pub entity_id: VarInt,
    pub position: BlockPos,
    pub destroy_stage: u8,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::BLOCK_ENTITY_UPDATE_S2C)]
pub struct BlockEntityUpdateS2c<'a> {
    pub position: BlockPos,
    pub kind: VarInt,
    pub data: Cow<'a, Compound>,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::BLOCK_EVENT_S2C)]
pub struct BlockEventS2c {
    pub position: BlockPos,
    pub action_id: u8,
    pub action_parameter: u8,
    pub block_type: VarInt,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::BLOCK_UPDATE_S2C)]
pub struct BlockUpdateS2c {
    pub position: BlockPos,
    pub block_id: VarInt,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::EXPLOSION_S2C)]
pub struct ExplosionS2c<'a> {
    pub window_id: u8,
    pub recipe: Ident<Cow<'a, str>>,
    pub make_all: bool,
}
