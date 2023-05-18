use valence_core::protocol::var_long::VarLong;
use valence_core::protocol::var_int::VarInt;

#[derive(Clone, Debug, Encode, Decode)]
pub struct WorldBorderCenterChangedS2c {
    pub x_pos: f64,
    pub z_pos: f64,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct WorldBorderInitializeS2c {
    pub x: f64,
    pub z: f64,
    pub old_diameter: f64,
    pub new_diameter: f64,
    pub speed: VarLong,
    pub portal_teleport_boundary: VarInt,
    pub warning_blocks: VarInt,
    pub warning_time: VarInt,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct WorldBorderInterpolateSizeS2c {
    pub old_diameter: f64,
    pub new_diameter: f64,
    pub speed: VarLong,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct WorldBorderSizeChangedS2c {
    pub diameter: f64,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct WorldBorderWarningBlocksChangedS2c {
    pub warning_blocks: VarInt,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct WorldBorderWarningTimeChangedS2c {
    pub warning_time: VarInt,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct WorldEventS2c {
    pub event: i32,
    pub location: BlockPos,
    pub data: i32,
    pub disable_relative_volume: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct WorldTimeUpdateS2c {
    /// The age of the world in 1/20ths of a second.
    pub world_age: i64,
    /// The current time of day in 1/20ths of a second.
    /// The value should be in the range \[0, 24000].
    /// 6000 is noon, 12000 is sunset, and 18000 is midnight.
    pub time_of_day: i64,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ChunkBiomeDataS2c<'a> {
    pub chunks: Cow<'a, [ChunkBiome<'a>]>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ChunkBiome<'a> {
    pub pos: ChunkPos,
    /// Chunk data structure, with sections containing only the `Biomes` field.
    pub data: &'a [u8],
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ChunkDataS2c<'a> {
    pub pos: ChunkPos,
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
    pub kind: VarInt,
    pub data: Cow<'a, Compound>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ChunkDeltaUpdateS2c<'a> {
    pub chunk_section_position: i64,
    pub invert_trust_edges: bool,
    pub blocks: Cow<'a, [VarLong]>,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ChunkLoadDistanceS2c {
    pub view_distance: VarInt,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct UnloadChunkS2c {
    pub pos: ChunkPos,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ChunkRenderDistanceCenterS2c {
    pub chunk_x: VarInt,
    pub chunk_z: VarInt,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct LightUpdateS2c {
    pub chunk_x: VarInt,
    pub chunk_z: VarInt,
    pub trust_edges: bool,
    pub sky_light_mask: Vec<u64>,
    pub block_light_mask: Vec<u64>,
    pub empty_sky_light_mask: Vec<u64>,
    pub empty_block_light_mask: Vec<u64>,
    pub sky_light_arrays: Vec<LengthPrefixedArray<u8, 2048>>,
    pub block_light_arrays: Vec<LengthPrefixedArray<u8, 2048>>,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct BlockBreakingProgressS2c {
    pub entity_id: VarInt,
    pub position: BlockPos,
    pub destroy_stage: u8,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct BlockEntityUpdateS2c<'a> {
    pub position: BlockPos,
    pub kind: VarInt,
    pub data: Cow<'a, Compound>,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct BlockEventS2c {
    pub position: BlockPos,
    pub action_id: u8,
    pub action_parameter: u8,
    pub block_type: VarInt,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct BlockUpdateS2c {
    pub position: BlockPos,
    pub block_id: VarInt,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ExplosionS2c<'a> {
    pub window_id: u8,
    pub recipe: Ident<Cow<'a, str>>,
    pub make_all: bool,
}
