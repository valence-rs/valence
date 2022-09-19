//! Chunks and related types.
//!
//! A chunk is a 16x16-block segment of a world with a height determined by the
//! [`Dimension`](crate::dimension::Dimension) of the world.
//!
//! In addition to blocks, chunks also contain [biomes](crate::biome::Biome).
//! Every 4x4x4 segment of blocks in a chunk corresponds to a biome.

// TODO: https://github.com/rust-lang/rust/issues/88581 for div_ceil

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io::Write;
use std::iter::FusedIterator;

use bitvec::vec::BitVec;
use num::Integer;
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::biome::BiomeId;
use crate::block::BlockState;
use crate::block_pos::BlockPos;
pub use crate::chunk_pos::ChunkPos;
use crate::config::Config;
use crate::dimension::DimensionId;
use crate::protocol::packets::s2c::play::{
    BlockUpdate, ChunkDataAndUpdateLight, ChunkDataHeightmaps, S2cPlayPacket, UpdateSectionBlocks,
};
use crate::protocol::{Encode, NbtBridge, VarInt, VarLong};
use crate::server::SharedServer;

/// A container for all [`LoadedChunk`]s in a [`World`](crate::world::World).
pub struct Chunks<C: Config> {
    chunks: HashMap<ChunkPos, LoadedChunk<C>>,
    shared: SharedServer<C>,
    dimension: DimensionId,
}

impl<C: Config> Chunks<C> {
    pub(crate) fn new(shared: SharedServer<C>, dimension: DimensionId) -> Self {
        Self {
            chunks: HashMap::new(),
            shared,
            dimension,
        }
    }

    /// Consumes an [`UnloadedChunk`] and creates a [`LoadedChunk`] at a given
    /// position. An exclusive reference to the new chunk is returned.
    ///
    /// If a chunk at the position already exists, then the old chunk
    /// is overwritten and its contents are dropped.
    ///
    /// The given chunk is resized to match the height of the world as if by
    /// calling [`UnloadedChunk::resize`].
    ///
    /// **Note**: For the vanilla Minecraft client to see a chunk, all chunks
    /// adjacent to it must also be loaded. Clients should not be spawned within
    /// unloaded chunks via [`spawn`](crate::client::Client::spawn).
    pub fn insert(
        &mut self,
        pos: impl Into<ChunkPos>,
        chunk: UnloadedChunk,
        state: C::ChunkState,
    ) -> &mut LoadedChunk<C> {
        let dimension_section_count = (self.shared.dimension(self.dimension).height / 16) as usize;
        let biome_registry_len = self.shared.biomes().len();
        let loaded = LoadedChunk::new(chunk, dimension_section_count, biome_registry_len, state);

        match self.chunks.entry(pos.into()) {
            Entry::Occupied(mut oe) => {
                oe.insert(loaded);
                oe.into_mut()
            }
            Entry::Vacant(ve) => ve.insert(loaded),
        }
    }

    /// Removes a chunk at the provided position.
    ///
    /// If a chunk exists at the position, then it is removed from the world and
    /// its content is returned. Otherwise, `None` is returned.
    pub fn remove(&mut self, pos: impl Into<ChunkPos>) -> Option<(UnloadedChunk, C::ChunkState)> {
        let loaded = self.chunks.remove(&pos.into())?;

        let unloaded = UnloadedChunk {
            sections: loaded.sections.into(),
        };

        Some((unloaded, loaded.state))
    }

    /// Returns the number of loaded chunks.
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Returns `true` if there are no loaded chunks.
    pub fn is_empty(&self) -> bool {
        self.chunks.len() == 0
    }

    /// Gets a shared reference to the chunk at the provided position.
    ///
    /// If there is no chunk at the position, then `None` is returned.
    pub fn get(&self, pos: impl Into<ChunkPos>) -> Option<&LoadedChunk<C>> {
        self.chunks.get(&pos.into())
    }

    /// Gets an exclusive reference to the chunk at the provided position.
    ///
    /// If there is no chunk at the position, then `None` is returned.
    pub fn get_mut(&mut self, pos: impl Into<ChunkPos>) -> Option<&mut LoadedChunk<C>> {
        self.chunks.get_mut(&pos.into())
    }

    /// Removes all chunks for which `f` returns `false`.
    ///
    /// All chunks are visited in an unspecified order.
    pub fn retain(&mut self, mut f: impl FnMut(ChunkPos, &mut LoadedChunk<C>) -> bool) {
        self.chunks.retain(|&pos, chunk| f(pos, chunk))
    }

    /// Deletes all chunks.
    pub fn clear(&mut self) {
        self.chunks.clear();
    }

    /// Returns an iterator over all chunks in the world in an unspecified
    /// order.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (ChunkPos, &LoadedChunk<C>)> + FusedIterator + Clone + '_
    {
        self.chunks.iter().map(|(&pos, chunk)| (pos, chunk))
    }

    /// Returns a mutable iterator over all chunks in the world in an
    /// unspecified order.
    pub fn iter_mut(
        &mut self,
    ) -> impl ExactSizeIterator<Item = (ChunkPos, &mut LoadedChunk<C>)> + FusedIterator + '_ {
        self.chunks.iter_mut().map(|(&pos, chunk)| (pos, chunk))
    }

    /// Returns a parallel iterator over all chunks in the world in an
    /// unspecified order.
    pub fn par_iter(
        &self,
    ) -> impl ParallelIterator<Item = (ChunkPos, &LoadedChunk<C>)> + Clone + '_ {
        self.chunks.par_iter().map(|(&pos, chunk)| (pos, chunk))
    }

    /// Returns a parallel mutable iterator over all chunks in the world in an
    /// unspecified order.
    pub fn par_iter_mut(
        &mut self,
    ) -> impl ParallelIterator<Item = (ChunkPos, &mut LoadedChunk<C>)> + '_ {
        self.chunks.par_iter_mut().map(|(&pos, chunk)| (pos, chunk))
    }

    /// Gets the block state at an absolute block position in world space.
    ///
    /// If the position is not inside of a chunk, then `None` is returned.
    ///
    /// **Note**: if you need to get a large number of blocks, it is more
    /// efficient to read from the chunks directly with
    /// [`Chunk::get_block_state`].
    pub fn get_block_state(&self, pos: impl Into<BlockPos>) -> Option<BlockState> {
        let pos = pos.into();
        let chunk_pos = ChunkPos::from(pos);

        let chunk = self.get(chunk_pos)?;

        let min_y = self.shared.dimension(self.dimension).min_y;

        let y = pos.y.checked_sub(min_y)?.try_into().ok()?;

        if y < chunk.height() {
            Some(chunk.get_block_state(
                pos.x.rem_euclid(16) as usize,
                y,
                pos.z.rem_euclid(16) as usize,
            ))
        } else {
            None
        }
    }

    /// Sets the block state at an absolute block position in world space.
    ///
    /// If the position is inside of a chunk, then `true` is returned and the
    /// block is set. Otherwise, `false` is returned and the function has no
    /// effect.
    ///
    /// **Note**: if you need to set a large number of blocks, it may be more
    /// efficient write to the chunks directly with
    /// [`Chunk::set_block_state`].
    pub fn set_block_state(&mut self, pos: impl Into<BlockPos>, block: BlockState) -> bool {
        let pos = pos.into();
        let chunk_pos = ChunkPos::from(pos);

        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            let min_y = self.shared.dimension(self.dimension).min_y;

            if let Some(y) = pos.y.checked_sub(min_y).and_then(|y| y.try_into().ok()) {
                if y < chunk.height() {
                    chunk.set_block_state(
                        pos.x.rem_euclid(16) as usize,
                        y,
                        pos.z.rem_euclid(16) as usize,
                        block,
                    );
                    return true;
                }
            }
        }

        false
    }

    /// Apply chunk modifications to only the chunks that were created this
    /// tick.
    pub(crate) fn update_created_this_tick(&mut self) {
        let biome_registry_len = self.shared.biomes().len();
        self.chunks.par_iter_mut().for_each(|(_, chunk)| {
            if chunk.created_this_tick() {
                chunk.apply_modifications(biome_registry_len);
            }
        });
    }

    /// Apply chunk modifications to all chunks and clear the created_this_tick
    /// flag.
    pub(crate) fn update(&mut self) {
        let biome_registry_len = self.shared.biomes().len();
        self.chunks.par_iter_mut().for_each(|(_, chunk)| {
            chunk.apply_modifications(biome_registry_len);
            chunk.created_this_tick = false;
        });
    }
}

/// Operations that can be performed on a chunk. [`LoadedChunk`] and
/// [`UnloadedChunk`] implement this trait.
pub trait Chunk {
    /// Returns the height of this chunk in blocks. The result is always a
    /// multiple of 16.
    fn height(&self) -> usize;

    /// Gets the block state at the provided offsets in the chunk.
    ///
    /// **Note**: The arguments to this function are offsets from the minimum
    /// corner of the chunk in _chunk space_ rather than _world space_. You
    /// might be looking for [`Chunks::get_block_state`] instead.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk.
    fn get_block_state(&self, x: usize, y: usize, z: usize) -> BlockState;

    /// Sets the block state at the provided offsets in the chunk.
    ///
    /// **Note**: The arguments to this function are offsets from the minimum
    /// corner of the chunk in _chunk space_ rather than _world space_. You
    /// might be looking for [`Chunks::set_block_state`] instead.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk.
    fn set_block_state(&mut self, x: usize, y: usize, z: usize, block: BlockState);

    /// Gets the biome at the provided biome offsets in the chunk.
    ///
    /// **Note**: the arguments are **not** block positions. Biomes are 4x4x4
    /// segments of a chunk, so `x` and `z` are in `0..=4`.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk.
    fn get_biome(&self, x: usize, y: usize, z: usize) -> BiomeId;

    /// Sets the biome at the provided biome offsets in the chunk.
    ///
    /// **Note**: the arguments are **not** block positions. Biomes are 4x4x4
    /// segments of a chunk, so `x` and `z` are in `0..=4`.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk.
    fn set_biome(&mut self, x: usize, y: usize, z: usize, biome: BiomeId);
}

/// A chunk that is not loaded in any world.
pub struct UnloadedChunk {
    sections: Vec<ChunkSection>,
    // TODO: block_entities: HashMap<u32, BlockEntity>,
}

impl UnloadedChunk {
    /// Constructs a new unloaded chunk containing only [`BlockState::AIR`] with
    /// the given height in blocks.
    ///
    /// # Panics
    ///
    /// Panics if the value of `height` does not meet the following criteria:
    /// `height % 16 == 0 && height <= 4064`.
    pub fn new(height: usize) -> Self {
        let mut chunk = Self {
            sections: Vec::new(),
        };

        chunk.resize(height);
        chunk
    }

    /// Changes the height of the chunk to `new_height`. This is a potentially
    /// expensive operation that may involve copying.
    ///
    /// The chunk is extended and truncated from the top. New blocks are always
    /// [`BlockState::AIR`].
    ///
    /// # Panics
    ///
    /// The constraints on `new_height` are the same as [`Self::new`].
    pub fn resize(&mut self, new_height: usize) {
        assert!(
            new_height % 16 == 0 && new_height <= 4064,
            "invalid chunk height of {new_height}"
        );

        let old_height = self.sections.len() * 16;

        if new_height > old_height {
            let sect = ChunkSection {
                blocks: [BlockState::AIR.to_raw(); 4096],
                modified_count: 0,
                biomes: [BiomeId::default(); 64],
                compact_data: Vec::new(),
            };

            let additional = (new_height - old_height) / 16;
            self.sections.reserve_exact(additional);
            self.sections.resize_with(new_height / 16, || sect.clone());
            debug_assert_eq!(self.sections.capacity(), self.sections.len());
        } else if new_height < old_height {
            self.sections.truncate(new_height / 16);
        }
    }
}

/// Constructs a new chunk with height `0`.
impl Default for UnloadedChunk {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Chunk for UnloadedChunk {
    fn height(&self) -> usize {
        self.sections.len() * 16
    }

    fn get_block_state(&self, x: usize, y: usize, z: usize) -> BlockState {
        assert!(
            x < 16 && y < self.height() && z < 16,
            "chunk block offsets must be within bounds"
        );

        BlockState::from_raw_unchecked(
            self.sections[y / 16].blocks[x + z * 16 + y % 16 * 16 * 16] & BLOCK_STATE_MASK,
        )
    }

    fn set_block_state(&mut self, x: usize, y: usize, z: usize, block: BlockState) {
        assert!(
            x < 16 && y < self.height() && z < 16,
            "chunk block offsets must be within bounds"
        );

        self.sections[y / 16].blocks[x + z * 16 + y % 16 * 16 * 16] = block.to_raw();
        // TODO: handle block entity here?
    }

    fn get_biome(&self, x: usize, y: usize, z: usize) -> BiomeId {
        assert!(
            x < 4 && y < self.height() / 4 && z < 4,
            "chunk biome offsets must be within bounds"
        );

        self.sections[y / 4].biomes[x + z * 4 + y % 4 * 4 * 4]
    }

    fn set_biome(&mut self, x: usize, y: usize, z: usize, biome: BiomeId) {
        assert!(
            x < 4 && y < self.height() / 4 && z < 4,
            "chunk biome offsets must be within bounds"
        );

        self.sections[y / 4].biomes[x + z * 4 + y % 4 * 4 * 4] = biome;
    }
}

/// A chunk which is currently loaded in a world.
pub struct LoadedChunk<C: Config> {
    /// Custom state.
    pub state: C::ChunkState,
    sections: Box<[ChunkSection]>,
    // TODO block_entities: HashMap<u32, BlockEntity>,
    /// The MOTION_BLOCKING heightmap
    heightmap: Vec<i64>,
    created_this_tick: bool,
}

/// A 16x16x16 section of blocks, biomes, and light in a chunk.
#[derive(Clone)]
struct ChunkSection {
    /// The block states in this section stored in x, z, y order.
    /// The most significant bit is used to indicate if this block has been
    /// modified.
    blocks: [u16; 4096],
    /// The number of modified blocks
    modified_count: u16,
    biomes: [BiomeId; 64],
    compact_data: Vec<u8>,
}

const BLOCK_STATE_MASK: u16 = 0x7fff;

const _: () = assert!(
    BlockState::max_raw() <= BLOCK_STATE_MASK,
    "There is not enough space in the block state type to store the modified bit. A bit array \
     separate from the block state array should be created to keep track of modified blocks in \
     the chunk section."
);

impl<C: Config> LoadedChunk<C> {
    fn new(
        mut chunk: UnloadedChunk,
        dimension_section_count: usize,
        biome_registry_len: usize,
        state: C::ChunkState,
    ) -> Self {
        chunk.resize(dimension_section_count * 16);

        let mut sections = chunk.sections.into_boxed_slice();

        // Mark all sections as modified so the chunk is properly initialized.
        for sect in sections.iter_mut() {
            sect.modified_count = 1;
        }

        let mut loaded = Self {
            state,
            sections,
            heightmap: Vec::new(),
            created_this_tick: true,
        };

        loaded.apply_modifications(biome_registry_len);
        loaded
    }

    /// Returns `true` if this chunk was created during the current tick.
    pub fn created_this_tick(&self) -> bool {
        self.created_this_tick
    }

    /// Gets the chunk data packet for this chunk with the given position. This
    /// does not include unapplied changes.
    pub(crate) fn chunk_data_packet(&self, pos: ChunkPos) -> ChunkDataAndUpdateLight {
        let mut blocks_and_biomes = Vec::new();

        for sect in self.sections.iter() {
            blocks_and_biomes.extend_from_slice(&sect.compact_data);
        }

        ChunkDataAndUpdateLight {
            chunk_x: pos.x,
            chunk_z: pos.z,
            heightmaps: NbtBridge(ChunkDataHeightmaps {
                motion_blocking: self.heightmap.clone(),
            }),
            blocks_and_biomes,
            block_entities: Vec::new(), // TODO
            trust_edges: true,
            // sky_light_mask: bitvec![u64, _; 1; section_count + 2],
            sky_light_mask: BitVec::new(),
            block_light_mask: BitVec::new(),
            empty_sky_light_mask: BitVec::new(),
            empty_block_light_mask: BitVec::new(),
            // sky_light_arrays: vec![[0xff; 2048]; section_count + 2],
            sky_light_arrays: Vec::new(),
            block_light_arrays: Vec::new(),
        }
    }

    /// Returns unapplied changes to this chunk as block change packets through
    /// the provided closure.
    pub(crate) fn block_change_packets(
        &self,
        pos: ChunkPos,
        min_y: i32,
        mut push_packet: impl FnMut(BlockChangePacket),
    ) {
        for (sect_y, sect) in self.sections.iter().enumerate() {
            if sect.modified_count == 1 {
                let (idx, &block) = sect
                    .blocks
                    .iter()
                    .enumerate()
                    .find(|&(_, &b)| b & !BLOCK_STATE_MASK != 0)
                    .expect("invalid modified count");

                let global_x = pos.x * 16 + (idx % 16) as i32;
                let global_y = sect_y as i32 * 16 + (idx / (16 * 16)) as i32 + min_y;
                let global_z = pos.z * 16 + (idx / 16 % 16) as i32;

                push_packet(BlockChangePacket::Single(BlockUpdate {
                    location: BlockPos::new(global_x, global_y, global_z),
                    block_id: VarInt((block & BLOCK_STATE_MASK).into()),
                }));
            } else if sect.modified_count > 1 {
                let mut blocks = Vec::new();
                for y in 0..16 {
                    for z in 0..16 {
                        for x in 0..16 {
                            let block =
                                sect.blocks[x as usize + z as usize * 16 + y as usize * 16 * 16];

                            if block & !BLOCK_STATE_MASK != 0 {
                                blocks.push(VarLong(
                                    ((block & BLOCK_STATE_MASK) as i64) << 12
                                        | (x << 8 | z << 4 | y),
                                ))
                            }
                        }
                    }
                }

                let chunk_section_position = (pos.x as i64) << 42
                    | (pos.z as i64 & 0x3fffff) << 20
                    | (sect_y as i64 + min_y.div_euclid(16) as i64) & 0xfffff;

                push_packet(BlockChangePacket::Multi(UpdateSectionBlocks {
                    chunk_section_position,
                    invert_trust_edges: false,
                    blocks,
                }));
            }
        }
    }

    fn apply_modifications(&mut self, biome_registry_len: usize) {
        let mut any_modified = false;

        for sect in self.sections.iter_mut() {
            if sect.modified_count > 0 {
                sect.modified_count = 0;
                any_modified = true;

                sect.compact_data.clear();

                let mut non_air_block_count: i16 = 0;

                for b in &mut sect.blocks {
                    *b &= BLOCK_STATE_MASK;
                    if !BlockState::from_raw_unchecked(*b).is_air() {
                        non_air_block_count += 1;
                    }
                }

                non_air_block_count.encode(&mut sect.compact_data).unwrap();

                encode_paletted_container(
                    sect.blocks.iter().cloned(),
                    4,
                    9,
                    log2_ceil(BlockState::max_raw() as usize),
                    &mut sect.compact_data,
                )
                .unwrap();

                encode_paletted_container(
                    sect.biomes.iter().map(|b| b.0),
                    0,
                    4,
                    log2_ceil(biome_registry_len),
                    &mut sect.compact_data,
                )
                .unwrap();
            }
        }

        if any_modified {
            build_heightmap(&self.sections, &mut self.heightmap);
        }
    }
}

impl<C: Config> Chunk for LoadedChunk<C> {
    fn height(&self) -> usize {
        self.sections.len() * 16
    }

    fn get_block_state(&self, x: usize, y: usize, z: usize) -> BlockState {
        assert!(
            x < 16 && y < self.height() && z < 16,
            "chunk block offsets must be within bounds"
        );

        BlockState::from_raw_unchecked(
            self.sections[y / 16].blocks[x + z * 16 + y % 16 * 16 * 16] & BLOCK_STATE_MASK,
        )
    }

    fn set_block_state(&mut self, x: usize, y: usize, z: usize, block: BlockState) {
        assert!(
            x < 16 && y < self.height() && z < 16,
            "chunk block offsets must be within bounds"
        );

        let sect = &mut self.sections[y / 16];
        let idx = x + z * 16 + y % 16 * 16 * 16;

        if block.to_raw() != sect.blocks[idx] & BLOCK_STATE_MASK {
            if sect.blocks[idx] & !BLOCK_STATE_MASK == 0 {
                sect.modified_count += 1;
            }
            sect.blocks[idx] = block.to_raw() | !BLOCK_STATE_MASK;

            // TODO: handle block entity here?
        }
    }

    fn get_biome(&self, x: usize, y: usize, z: usize) -> BiomeId {
        assert!(
            x < 4 && y < self.height() / 4 && z < 4,
            "chunk biome offsets must be within bounds"
        );

        self.sections[y / 4].biomes[x + z * 4 + y % 4 * 4 * 4]
    }

    fn set_biome(&mut self, x: usize, y: usize, z: usize, biome: BiomeId) {
        assert!(
            x < 4 && y < self.height() / 4 && z < 4,
            "chunk biome offsets must be within bounds"
        );

        self.sections[y / 4].biomes[x + z * 4 + y % 4 * 4 * 4] = biome;
    }
}

#[derive(Clone, Debug)]
pub(crate) enum BlockChangePacket {
    Single(BlockUpdate),
    Multi(UpdateSectionBlocks),
}

impl From<BlockChangePacket> for S2cPlayPacket {
    fn from(p: BlockChangePacket) -> Self {
        match p {
            BlockChangePacket::Single(p) => p.into(),
            BlockChangePacket::Multi(p) => p.into(),
        }
    }
}

/// Builds the MOTION_BLOCKING heightmap.
fn build_heightmap(sections: &[ChunkSection], heightmap: &mut Vec<i64>) {
    let height = sections.len() * 16;
    let bits_per_val = log2_ceil(height);
    let vals_per_u64 = 64 / bits_per_val;
    let u64_count = Integer::div_ceil(&256, &vals_per_u64);

    heightmap.clear();
    heightmap.resize(u64_count, 0);

    for x in 0..16 {
        for z in 0..16 {
            for y in (0..height).rev() {
                let block = BlockState::from_raw_unchecked(
                    sections[y / 16].blocks[x + z * 16 + y % 16 * 16 * 16] & BLOCK_STATE_MASK,
                );

                // TODO: is_solid || is_fluid heuristic for motion blocking.
                if !block.is_air() {
                    let column_height = y as u64;

                    let i = x * 16 + z; // TODO: X or Z major?
                    heightmap[i / vals_per_u64] |=
                        (column_height << (i % vals_per_u64 * bits_per_val)) as i64;

                    break;
                }
            }
        }
    }
}

fn encode_paletted_container(
    mut entries: impl ExactSizeIterator<Item = u16> + Clone,
    min_bits_per_idx: usize,
    direct_threshold: usize,
    direct_bits_per_idx: usize,
    w: &mut impl Write,
) -> anyhow::Result<()> {
    let mut palette = Vec::new();

    for entry in entries.clone() {
        if !palette.contains(&entry) {
            palette.push(entry);
        }
    }

    let bits_per_idx = log2_ceil(palette.len());

    (bits_per_idx as u8).encode(w)?;

    if bits_per_idx == 0 {
        // Single value case
        debug_assert_eq!(palette.len(), 1);
        VarInt(palette[0] as i32).encode(w)?;
        VarInt(0).encode(w)?; // data array length
    } else if bits_per_idx >= direct_threshold {
        // Direct case
        // Skip the palette
        let idxs_per_u64 = 64 / direct_bits_per_idx;
        let u64_count = Integer::div_ceil(&entries.len(), &idxs_per_u64);

        VarInt(u64_count as i32).encode(w)?;

        for _ in 0..idxs_per_u64 {
            let mut val = 0u64;
            for i in 0..idxs_per_u64 {
                if let Some(entry) = entries.next() {
                    val |= (entry as u64) << (i * direct_bits_per_idx);
                }
            }
            val.encode(w)?;
        }
    } else {
        // Indirect case
        VarInt(palette.len() as i32).encode(w)?;
        for &val in &palette {
            VarInt(val as i32).encode(w)?;
        }

        let bits_per_idx = bits_per_idx.max(min_bits_per_idx);
        let idxs_per_u64 = 64 / bits_per_idx;
        let u64_count = Integer::div_ceil(&entries.len(), &idxs_per_u64);

        VarInt(u64_count as i32).encode(w)?;

        for _ in 0..u64_count {
            let mut val = 0u64;
            for i in 0..idxs_per_u64 {
                if let Some(entry) = entries.next() {
                    let palette_idx = palette
                        .iter()
                        .position(|&e| e == entry)
                        .expect("entry should be in the palette")
                        as u64;

                    val |= palette_idx << (i * bits_per_idx);
                }
            }
            val.encode(w)?;
        }
    }

    Ok(())
}

/// Calculates the log base 2 rounded up.
fn log2_ceil(n: usize) -> usize {
    debug_assert_ne!(n, 0);
    n.next_power_of_two().trailing_zeros() as usize
}
