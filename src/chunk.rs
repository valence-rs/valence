//! Chunks and related types.
//!
//! A chunk is a 16x16-block segment of a world with a height determined by the
//! [`Dimension`](crate::dimension::Dimension) of the world.
//!
//! In addition to blocks, chunks also contain [biomes](crate::biome::Biome).
//! Every 4x4x4 segment of blocks in a chunk corresponds to a biome.

use std::collections::hash_map::Entry;
use std::io::Write;
use std::iter::FusedIterator;
use std::mem;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::sync::{Mutex, MutexGuard};

use entity_partition::PartitionCell;
use paletted_container::PalettedContainer;
pub use pos::ChunkPos;
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};
use rustc_hash::FxHashMap;
use valence_nbt::compound;
use valence_protocol::packets::s2c::play::{
    BlockUpdate, ChunkDataAndUpdateLightEncode, UpdateSectionBlocksEncode,
};
use valence_protocol::{BlockPos, BlockState, Encode, LengthPrefixedArray, VarInt, VarLong};

use crate::biome::BiomeId;
use crate::config::Config;
use crate::packet::{PacketWriter, WritePacket};
use crate::util::bit_width;

pub(crate) mod entity_partition;
mod paletted_container;
mod pos;

/// A container for all [`LoadedChunk`]s in a [`World`](crate::world::World).
pub struct Chunks<C: Config> {
    /// Maps chunk positions to chunks. We store both loaded chunks and
    /// partition cells here so we can get both in a single hashmap lookup
    /// during the client update procedure.
    chunks: FxHashMap<ChunkPos, (Option<LoadedChunk<C>>, PartitionCell)>,
    dimension_height: i32,
    dimension_min_y: i32,
    filler_sky_light_mask: Box<[u64]>,
    /// Sending filler light data causes the vanilla client to lag
    /// less. Hopefully we can remove this in the future.
    filler_sky_light_arrays: Box<[LengthPrefixedArray<u8, 2048>]>,
    biome_registry_len: usize,
    compression_threshold: Option<u32>,
}

impl<C: Config> Chunks<C> {
    pub(crate) fn new(
        dimension_height: i32,
        dimension_min_y: i32,
        biome_registry_len: usize,
        compression_threshold: Option<u32>,
    ) -> Self {
        let section_count = (dimension_height / 16 + 2) as usize;

        let mut sky_light_mask = vec![0; num::Integer::div_ceil(&section_count, &16)];

        for i in 0..section_count {
            sky_light_mask[i / 64] |= 1 << (i % 64);
        }

        Self {
            chunks: FxHashMap::default(),
            dimension_height,
            dimension_min_y,
            filler_sky_light_mask: sky_light_mask.into(),
            filler_sky_light_arrays: vec![LengthPrefixedArray([0xff; 2048]); section_count].into(),
            biome_registry_len,
            compression_threshold,
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
    /// unloaded chunks via [`respawn`](crate::client::Client::respawn).
    pub fn insert(
        &mut self,
        pos: impl Into<ChunkPos>,
        chunk: UnloadedChunk,
        state: C::ChunkState,
    ) -> &mut LoadedChunk<C> {
        let dimension_section_count = (self.dimension_height / 16) as usize;
        let loaded = LoadedChunk::new(chunk, dimension_section_count, state);

        match self.chunks.entry(pos.into()) {
            Entry::Occupied(mut oe) => {
                oe.get_mut().0 = Some(loaded);
                oe.into_mut().0.as_mut().unwrap()
            }
            Entry::Vacant(ve) => ve
                .insert((Some(loaded), PartitionCell::new()))
                .0
                .as_mut()
                .unwrap(),
        }
    }

    /// Returns the height of all loaded chunks in the world. This returns the
    /// same value as [`Chunk::section_count`] multiplied by 16 for all loaded
    /// chunks.
    pub fn height(&self) -> usize {
        self.dimension_height as usize
    }

    /// The minimum Y coordinate in world space that chunks in this world can
    /// occupy. This is relevant for [`Chunks::block_state`] and
    /// [`Chunks::set_block_state`].
    pub fn min_y(&self) -> i32 {
        self.dimension_min_y
    }

    /// Gets a shared reference to the chunk at the provided position.
    ///
    /// If there is no chunk at the position, then `None` is returned.
    pub fn get(&self, pos: impl Into<ChunkPos>) -> Option<&LoadedChunk<C>> {
        self.chunks.get(&pos.into())?.0.as_ref()
    }

    pub(crate) fn chunk_and_cell(
        &self,
        pos: ChunkPos,
    ) -> Option<&(Option<LoadedChunk<C>>, PartitionCell)> {
        self.chunks.get(&pos)
    }

    fn cell_mut(&mut self, pos: ChunkPos) -> Option<&mut PartitionCell> {
        self.chunks.get_mut(&pos).map(|(_, cell)| cell)
    }

    /// Gets an exclusive reference to the chunk at the provided position.
    ///
    /// If there is no chunk at the position, then `None` is returned.
    pub fn get_mut(&mut self, pos: impl Into<ChunkPos>) -> Option<&mut LoadedChunk<C>> {
        self.chunks.get_mut(&pos.into())?.0.as_mut()
    }

    /// Returns an iterator over all chunks in the world in an unspecified
    /// order.
    pub fn iter(&self) -> impl FusedIterator<Item = (ChunkPos, &LoadedChunk<C>)> + Clone + '_ {
        self.chunks
            .iter()
            .filter_map(|(&pos, (chunk, _))| chunk.as_ref().map(|c| (pos, c)))
    }

    /// Returns a mutable iterator over all chunks in the world in an
    /// unspecified order.
    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (ChunkPos, &mut LoadedChunk<C>)> + '_ {
        self.chunks
            .iter_mut()
            .filter_map(|(&pos, (chunk, _))| chunk.as_mut().map(|c| (pos, c)))
    }

    fn cells_mut(
        &mut self,
    ) -> impl ExactSizeIterator<Item = &mut PartitionCell> + FusedIterator + '_ {
        self.chunks.iter_mut().map(|(_, (_, cell))| cell)
    }

    /// Returns a parallel iterator over all chunks in the world in an
    /// unspecified order.
    pub fn par_iter(
        &self,
    ) -> impl ParallelIterator<Item = (ChunkPos, &LoadedChunk<C>)> + Clone + '_ {
        self.chunks
            .par_iter()
            .filter_map(|(&pos, (chunk, _))| chunk.as_ref().map(|c| (pos, c)))
    }

    /// Returns a parallel mutable iterator over all chunks in the world in an
    /// unspecified order.
    pub fn par_iter_mut(
        &mut self,
    ) -> impl ParallelIterator<Item = (ChunkPos, &mut LoadedChunk<C>)> + '_ {
        self.chunks
            .par_iter_mut()
            .filter_map(|(&pos, (chunk, _))| chunk.as_mut().map(|c| (pos, c)))
    }

    /// Gets the block state at an absolute block position in world space.
    ///
    /// If the position is not inside of a chunk, then `None` is returned.
    ///
    /// **Note**: if you need to get a large number of blocks, it is more
    /// efficient to read from the chunks directly with
    /// [`Chunk::block_state`].
    pub fn block_state(&self, pos: impl Into<BlockPos>) -> Option<BlockState> {
        let pos = pos.into();
        let chunk_pos = ChunkPos::from(pos);

        let chunk = self.get(chunk_pos)?;

        let y = pos.y.checked_sub(self.dimension_min_y)?.try_into().ok()?;

        if y < chunk.section_count() * 16 {
            Some(chunk.block_state(
                pos.x.rem_euclid(16) as usize,
                y,
                pos.z.rem_euclid(16) as usize,
            ))
        } else {
            None
        }
    }

    /// Sets the block state at an absolute block position in world space. The
    /// previous block state at the position is returned.
    ///
    /// If the given position is not inside of a loaded chunk, then a new chunk
    /// is created at the position before the block is set.
    ///
    /// If the position is completely out of bounds, then no new chunk is
    /// created and [`BlockState::AIR`] is returned.
    pub fn set_block_state(&mut self, pos: impl Into<BlockPos>, block: BlockState) -> BlockState
    where
        C::ChunkState: Default,
    {
        let pos = pos.into();

        let Some(y) = pos.y.checked_sub(self.dimension_min_y).and_then(|y| y.try_into().ok()) else {
            return BlockState::AIR;
        };

        if y >= self.dimension_height as usize {
            return BlockState::AIR;
        }

        let chunk = match self.chunks.entry(ChunkPos::from(pos)) {
            Entry::Occupied(oe) => oe.into_mut().0.get_or_insert_with(|| {
                let dimension_section_count = (self.dimension_height / 16) as usize;
                LoadedChunk::new(
                    UnloadedChunk::default(),
                    dimension_section_count,
                    Default::default(),
                )
            }),
            Entry::Vacant(ve) => {
                let dimension_section_count = (self.dimension_height / 16) as usize;
                let loaded = LoadedChunk::new(
                    UnloadedChunk::default(),
                    dimension_section_count,
                    Default::default(),
                );

                ve.insert((Some(loaded), PartitionCell::new()))
                    .0
                    .as_mut()
                    .unwrap()
            }
        };

        chunk.set_block_state(
            pos.x.rem_euclid(16) as usize,
            y,
            pos.z.rem_euclid(16) as usize,
            block,
        )
    }

    pub(crate) fn update_caches(&mut self) {
        let min_y = self.dimension_min_y;

        self.chunks.par_iter_mut().for_each(|(&pos, (chunk, _))| {
            let Some(chunk) = chunk else {
                // There is no chunk at this position.
                return;
            };

            if chunk.deleted {
                // Deleted chunks are not sending packets to anyone.
                return;
            }

            let mut compression_scratch = vec![];
            let mut blocks = vec![];

            chunk.cached_update_packets.clear();
            let mut any_blocks_modified = false;

            for (sect_y, sect) in chunk.sections.iter_mut().enumerate() {
                let modified_blocks_count: u32 = sect
                    .modified_blocks
                    .iter()
                    .map(|&bits| bits.count_ones())
                    .sum();

                // If the chunk is created this tick, clients are only going to be sent the
                // chunk data packet so there is no need to cache the modified blocks packets.
                if !chunk.created_this_tick {
                    if modified_blocks_count == 1 {
                        let (i, bits) = sect
                            .modified_blocks
                            .iter()
                            .cloned()
                            .enumerate()
                            .find(|(_, n)| *n > 0)
                            .expect("invalid modified count");

                        debug_assert_eq!(bits.count_ones(), 1);

                        let idx = i * USIZE_BITS + bits.trailing_zeros() as usize;
                        let block = sect.block_states.get(idx);

                        let global_x = pos.x * 16 + (idx % 16) as i32;
                        let global_y = sect_y as i32 * 16 + (idx / (16 * 16)) as i32 + min_y;
                        let global_z = pos.z * 16 + (idx / 16 % 16) as i32;

                        let mut writer = PacketWriter::new(
                            &mut chunk.cached_update_packets,
                            self.compression_threshold,
                            &mut compression_scratch,
                        );

                        writer
                            .write_packet(&BlockUpdate {
                                position: BlockPos::new(global_x, global_y, global_z),
                                block_id: VarInt(block.to_raw() as _),
                            })
                            .unwrap();
                    } else if modified_blocks_count > 1 {
                        blocks.clear();

                        for y in 0..16 {
                            for z in 0..16 {
                                for x in 0..16 {
                                    let idx = x as usize + z as usize * 16 + y as usize * 16 * 16;

                                    if sect.is_block_modified(idx) {
                                        let block_id = sect.block_states.get(idx).to_raw();
                                        let compact =
                                            (block_id as i64) << 12 | (x << 8 | z << 4 | y);

                                        blocks.push(VarLong(compact));
                                    }
                                }
                            }
                        }

                        let chunk_section_position = (pos.x as i64) << 42
                            | (pos.z as i64 & 0x3fffff) << 20
                            | (sect_y as i64 + min_y.div_euclid(16) as i64) & 0xfffff;

                        let mut writer = PacketWriter::new(
                            &mut chunk.cached_update_packets,
                            self.compression_threshold,
                            &mut compression_scratch,
                        );

                        writer
                            .write_packet(&UpdateSectionBlocksEncode {
                                chunk_section_position,
                                invert_trust_edges: false,
                                blocks: &blocks,
                            })
                            .unwrap();
                    }
                }

                if modified_blocks_count > 0 {
                    any_blocks_modified = true;
                    sect.modified_blocks.fill(0);
                }
            }

            // Clear the cache if the cache was invalidated.
            if any_blocks_modified || chunk.any_biomes_modified {
                chunk.any_biomes_modified = false;
                chunk.cached_init_packet.get_mut().unwrap().clear();
            }

            // Initialize the chunk data cache on new chunks here so this work can be done
            // in parallel.
            if chunk.created_this_tick() {
                debug_assert!(chunk.cached_init_packet.get_mut().unwrap().is_empty());

                let _unused: MutexGuard<_> = chunk.get_chunk_data_packet(
                    &mut compression_scratch,
                    pos,
                    self.biome_registry_len,
                    &self.filler_sky_light_mask,
                    &self.filler_sky_light_arrays,
                    self.compression_threshold,
                );
            }
        });
    }

    /// Clears changes to partition cells and removes deleted chunks and
    /// partition cells.
    pub(crate) fn update(&mut self) {
        self.chunks.retain(|_, (chunk_opt, cell)| {
            if let Some(chunk) = chunk_opt {
                if chunk.deleted {
                    *chunk_opt = None;
                } else {
                    chunk.created_this_tick = false;
                }
            }

            cell.clear_incoming_outgoing();

            chunk_opt.is_some() || cell.entities().len() > 0
        });
    }
}

impl<C: Config, P: Into<ChunkPos>> Index<P> for Chunks<C> {
    type Output = LoadedChunk<C>;

    fn index(&self, index: P) -> &Self::Output {
        let ChunkPos { x, z } = index.into();
        self.get((x, z))
            .unwrap_or_else(|| panic!("missing chunk at ({x}, {z})"))
    }
}

impl<C: Config, P: Into<ChunkPos>> IndexMut<P> for Chunks<C> {
    fn index_mut(&mut self, index: P) -> &mut Self::Output {
        let ChunkPos { x, z } = index.into();
        self.get_mut((x, z))
            .unwrap_or_else(|| panic!("missing chunk at ({x}, {z})"))
    }
}

/// Operations that can be performed on a chunk. [`LoadedChunk`] and
/// [`UnloadedChunk`] implement this trait.
pub trait Chunk {
    /// Returns the number of sections in this chunk. To get the height of the
    /// chunk in meters, multiply the result by 16.
    fn section_count(&self) -> usize;

    /// Gets the block state at the provided offsets in the chunk.
    ///
    /// **Note**: The arguments to this function are offsets from the minimum
    /// corner of the chunk in _chunk space_ rather than _world space_.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk. `x` and `z`
    /// must be less than 16 while `y` must be less than `section_count() * 16`.
    fn block_state(&self, x: usize, y: usize, z: usize) -> BlockState;

    /// Sets the block state at the provided offsets in the chunk. The previous
    /// block state at the position is returned.
    ///
    /// **Note**: The arguments to this function are offsets from the minimum
    /// corner of the chunk in _chunk space_ rather than _world space_.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk. `x` and `z`
    /// must be less than 16 while `y` must be less than `section_count() * 16`.
    fn set_block_state(&mut self, x: usize, y: usize, z: usize, block: BlockState) -> BlockState;

    /// Sets every block in a section to the given block state.
    ///
    /// This is semantically equivalent to setting every block in the section
    /// with [`set_block_state`]. However, this function may be implemented more
    /// efficiently.
    ///
    /// # Panics
    ///
    /// Panics if `sect_y` is out of bounds. `sect_y` must be less than the
    /// section count.
    ///
    /// [`set_block_state`]: Self::set_block_state
    fn fill_block_states(&mut self, sect_y: usize, block: BlockState);

    /// Gets the biome at the provided biome offsets in the chunk.
    ///
    /// **Note**: the arguments are **not** block positions. Biomes are 4x4x4
    /// segments of a chunk, so `x` and `z` are in `0..4`.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk. `x` and `z`
    /// must be less than 4 while `y` must be less than `section_count() * 4`.
    fn biome(&self, x: usize, y: usize, z: usize) -> BiomeId;

    /// Sets the biome at the provided offsets in the chunk. The previous
    /// biome at the position is returned.
    ///
    /// **Note**: the arguments are **not** block positions. Biomes are 4x4x4
    /// segments of a chunk, so `x` and `z` are in `0..4`.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk. `x` and `z`
    /// must be less than 4 while `y` must be less than `section_count() * 4`.
    fn set_biome(&mut self, x: usize, y: usize, z: usize, biome: BiomeId) -> BiomeId;

    /// Sets every biome in a section to the given block state.
    ///
    /// This is semantically equivalent to setting every biome in the section
    /// with [`set_biome`]. However, this function may be implemented more
    /// efficiently.
    ///
    /// # Panics
    ///
    /// Panics if `sect_y` is out of bounds. `sect_y` must be less than the
    /// section count.
    ///
    /// [`set_biome`]: Self::set_biome
    fn fill_biomes(&mut self, sect_y: usize, biome: BiomeId);

    /// Optimizes this chunk to use the minimum amount of memory possible. It
    /// should have no observable effect on the contents of the chunk.
    ///
    /// This is a potentially expensive operation. The function is most
    /// effective when a large number of blocks and biomes have changed states.
    fn optimize(&mut self);
}

/// A chunk that is not loaded in any world.
pub struct UnloadedChunk {
    sections: Vec<ChunkSection>,
    // TODO: block_entities: BTreeMap<u32, BlockEntity>,
}

impl UnloadedChunk {
    /// Constructs a new unloaded chunk containing only [`BlockState::AIR`] and
    /// [`BiomeId::default()`] with the given number of sections. A section is a
    /// 16x16x16 meter volume.
    pub fn new(section_count: usize) -> Self {
        let mut chunk = Self { sections: vec![] };
        chunk.resize(section_count);
        chunk
    }

    /// Changes the section count of the chunk to `new_section_count`. This is a
    /// potentially expensive operation that may involve copying.
    ///
    /// The chunk is extended and truncated from the top. New blocks are always
    /// [`BlockState::AIR`] and biomes are [`BiomeId::default()`].
    pub fn resize(&mut self, new_section_count: usize) {
        let old_section_count = self.section_count();

        if new_section_count > old_section_count {
            self.sections
                .reserve_exact(new_section_count - old_section_count);
            self.sections
                .resize_with(new_section_count, ChunkSection::default);
            debug_assert_eq!(self.sections.capacity(), self.sections.len());
        } else {
            self.sections.truncate(new_section_count);
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
    fn section_count(&self) -> usize {
        self.sections.len()
    }

    fn block_state(&self, x: usize, y: usize, z: usize) -> BlockState {
        assert!(
            x < 16 && y < self.section_count() * 16 && z < 16,
            "chunk block offsets of ({x}, {y}, {z}) are out of bounds"
        );

        self.sections[y / 16]
            .block_states
            .get(x + z * 16 + y % 16 * 16 * 16)
    }

    fn set_block_state(&mut self, x: usize, y: usize, z: usize, block: BlockState) -> BlockState {
        assert!(
            x < 16 && y < self.section_count() * 16 && z < 16,
            "chunk block offsets of ({x}, {y}, {z}) are out of bounds"
        );

        let mut sect = &mut self.sections[y / 16];

        let old_block = sect.block_states.set(x + z * 16 + y % 16 * 16 * 16, block);

        match (block.is_air(), old_block.is_air()) {
            (true, false) => sect.non_air_count -= 1,
            (false, true) => sect.non_air_count += 1,
            _ => {}
        }

        old_block
    }

    fn fill_block_states(&mut self, sect_y: usize, block: BlockState) {
        let Some(sect) = self.sections.get_mut(sect_y) else {
            panic!(
                "section index {sect_y} out of bounds for chunk with {} sections",
                self.section_count()
            )
        };

        if block.is_air() {
            sect.non_air_count = 0;
        } else {
            sect.non_air_count = SECTION_BLOCK_COUNT as u16;
        }

        sect.block_states.fill(block);
    }

    fn biome(&self, x: usize, y: usize, z: usize) -> BiomeId {
        assert!(
            x < 4 && y < self.section_count() * 4 && z < 4,
            "chunk biome offsets of ({x}, {y}, {z}) are out of bounds"
        );

        self.sections[y / 4].biomes.get(x + z * 4 + y % 4 * 4 * 4)
    }

    fn set_biome(&mut self, x: usize, y: usize, z: usize, biome: BiomeId) -> BiomeId {
        assert!(
            x < 4 && y < self.section_count() * 4 && z < 4,
            "chunk biome offsets of ({x}, {y}, {z}) are out of bounds"
        );

        self.sections[y / 4]
            .biomes
            .set(x + z * 4 + y % 4 * 4 * 4, biome)
    }

    fn fill_biomes(&mut self, sect_y: usize, biome: BiomeId) {
        let Some(sect) = self.sections.get_mut(sect_y) else {
            panic!(
                "section index {sect_y} out of bounds for chunk with {} sections",
                self.section_count()
            )
        };

        sect.biomes.fill(biome);
    }

    fn optimize(&mut self) {
        for sect in self.sections.iter_mut() {
            sect.block_states.optimize();
            sect.biomes.optimize();
        }
    }
}

/// A chunk which is currently loaded in a world.
pub struct LoadedChunk<C: Config> {
    /// Custom state.
    pub state: C::ChunkState,
    sections: Box<[ChunkSection]>,
    // TODO: block_entities: BTreeMap<u32, BlockEntity>,
    cached_init_packet: Mutex<Vec<u8>>,
    cached_update_packets: Vec<u8>,
    /// If any of the biomes in this chunk were modified this tick.
    any_biomes_modified: bool,
    created_this_tick: bool,
    deleted: bool,
    /// For debugging purposes.
    #[cfg(debug_assertions)]
    uuid: uuid::Uuid,
}

impl<C: Config> Deref for LoadedChunk<C> {
    type Target = C::ChunkState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<C: Config> DerefMut for LoadedChunk<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

/// A 16x16x16 meter volume of blocks, biomes, and light in a chunk.
#[derive(Clone, Debug)]
struct ChunkSection {
    block_states: PalettedContainer<BlockState, SECTION_BLOCK_COUNT, { SECTION_BLOCK_COUNT / 2 }>,
    /// Contains a set bit for every block that has been modified in this
    /// section this tick. Ignored in unloaded chunks.
    modified_blocks: [usize; SECTION_BLOCK_COUNT / USIZE_BITS],
    /// Number of non-air blocks in this section.
    non_air_count: u16,
    biomes: PalettedContainer<BiomeId, 64, 32>,
}

// [T; 64] Doesn't implement Default so we can't derive :(
impl Default for ChunkSection {
    fn default() -> Self {
        Self {
            block_states: Default::default(),
            modified_blocks: [0; SECTION_BLOCK_COUNT / USIZE_BITS],
            non_air_count: 0,
            biomes: Default::default(),
        }
    }
}

const SECTION_BLOCK_COUNT: usize = 4096;
const USIZE_BITS: usize = usize::BITS as _;

impl ChunkSection {
    fn mark_block_as_modified(&mut self, idx: usize) {
        self.modified_blocks[idx / USIZE_BITS] |= 1 << (idx % USIZE_BITS);
    }

    fn mark_all_blocks_as_modified(&mut self) {
        self.modified_blocks.fill(usize::MAX);
    }

    fn is_block_modified(&self, idx: usize) -> bool {
        self.modified_blocks[idx / USIZE_BITS] >> (idx % USIZE_BITS) & 1 == 1
    }
}

impl<C: Config> LoadedChunk<C> {
    fn new(mut chunk: UnloadedChunk, dimension_section_count: usize, state: C::ChunkState) -> Self {
        chunk.resize(dimension_section_count);

        Self {
            state,
            sections: chunk.sections.into(),
            cached_init_packet: Mutex::new(vec![]),
            cached_update_packets: vec![],
            any_biomes_modified: false,
            created_this_tick: true,
            deleted: false,
            #[cfg(debug_assertions)]
            uuid: uuid::Uuid::from_u128(rand::random()),
        }
    }

    pub fn take(&mut self) -> UnloadedChunk {
        let unloaded = UnloadedChunk {
            sections: mem::take(&mut self.sections).into(),
        };

        self.created_this_tick = true;

        unloaded
    }

    /// Returns `true` if this chunk was created during the current tick.
    pub fn created_this_tick(&self) -> bool {
        self.created_this_tick
    }

    pub fn deleted(&self) -> bool {
        self.deleted
    }

    pub fn set_deleted(&mut self, deleted: bool) {
        self.deleted = deleted;
    }

    /// Queues the chunk data packet for this chunk with the given position.
    /// This will initialize the chunk for the client.
    pub(crate) fn write_chunk_data_packet(
        &self,
        mut writer: impl WritePacket,
        scratch: &mut Vec<u8>,
        pos: ChunkPos,
        chunks: &Chunks<C>,
    ) -> anyhow::Result<()> {
        #[cfg(debug_assertions)]
        assert_eq!(
            chunks[pos].uuid, self.uuid,
            "chunks and/or position arguments are incorrect"
        );

        let bytes = self.get_chunk_data_packet(
            scratch,
            pos,
            chunks.biome_registry_len,
            &chunks.filler_sky_light_mask,
            &chunks.filler_sky_light_arrays,
            chunks.compression_threshold,
        );

        writer.write_bytes(&bytes)
    }

    /// Gets the bytes of the cached chunk data packet, initializing the cache
    /// if it is empty.
    fn get_chunk_data_packet(
        &self,
        scratch: &mut Vec<u8>,
        pos: ChunkPos,
        biome_registry_len: usize,
        filler_sky_light_mask: &[u64],
        filler_sky_light_arrays: &[LengthPrefixedArray<u8, 2048>],
        compression_threshold: Option<u32>,
    ) -> MutexGuard<Vec<u8>> {
        let mut lck = self.cached_init_packet.lock().unwrap();

        if lck.is_empty() {
            scratch.clear();

            for sect in self.sections.iter() {
                sect.non_air_count.encode(&mut *scratch).unwrap();

                sect.block_states
                    .encode_mc_format(
                        &mut *scratch,
                        |b| b.to_raw().into(),
                        4,
                        8,
                        bit_width(BlockState::max_raw().into()),
                    )
                    .unwrap();

                sect.biomes
                    .encode_mc_format(
                        &mut *scratch,
                        |b| b.0.into(),
                        0,
                        3,
                        bit_width(biome_registry_len - 1),
                    )
                    .unwrap();
            }

            let mut compression_scratch = vec![];

            let mut writer =
                PacketWriter::new(&mut lck, compression_threshold, &mut compression_scratch);

            writer
                .write_packet(&ChunkDataAndUpdateLightEncode {
                    chunk_x: pos.x,
                    chunk_z: pos.z,
                    heightmaps: &compound! {
                        // TODO: MOTION_BLOCKING heightmap
                    },
                    blocks_and_biomes: scratch,
                    block_entities: &[],
                    trust_edges: true,
                    sky_light_mask: filler_sky_light_mask,
                    block_light_mask: &[],
                    empty_sky_light_mask: &[],
                    empty_block_light_mask: &[],
                    sky_light_arrays: filler_sky_light_arrays,
                    block_light_arrays: &[],
                })
                .unwrap();
        }

        lck
    }

    /// Queues block change packets for this chunk.
    pub(crate) fn write_block_change_packets(
        &self,
        mut writer: impl WritePacket,
    ) -> anyhow::Result<()> {
        writer.write_bytes(&self.cached_update_packets)
    }
}

impl<C: Config> Chunk for LoadedChunk<C> {
    fn section_count(&self) -> usize {
        self.sections.len()
    }

    fn block_state(&self, x: usize, y: usize, z: usize) -> BlockState {
        assert!(
            x < 16 && y < self.section_count() * 16 && z < 16,
            "chunk block offsets of ({x}, {y}, {z}) are out of bounds"
        );

        self.sections[y / 16]
            .block_states
            .get(x + z * 16 + y % 16 * 16 * 16)
    }

    fn set_block_state(&mut self, x: usize, y: usize, z: usize, block: BlockState) -> BlockState {
        assert!(
            x < 16 && y < self.section_count() * 16 && z < 16,
            "chunk block offsets of ({x}, {y}, {z}) are out of bounds"
        );

        let sect = &mut self.sections[y / 16];
        let idx = x + z * 16 + y % 16 * 16 * 16;

        let old_block = sect.block_states.set(idx, block);

        if block != old_block {
            match (block.is_air(), old_block.is_air()) {
                (true, false) => sect.non_air_count -= 1,
                (false, true) => sect.non_air_count += 1,
                _ => {}
            }

            sect.mark_block_as_modified(idx);
        }

        old_block
    }

    fn fill_block_states(&mut self, sect_y: usize, block: BlockState) {
        let Some(sect) = self.sections.get_mut(sect_y) else {
            panic!(
                "section index {sect_y} out of bounds for chunk with {} sections",
                self.section_count()
            )
        };

        // Mark the appropriate blocks as modified.
        // No need to iterate through all the blocks if we know they're all the same.
        if let PalettedContainer::Single(single) = &sect.block_states {
            if block != *single {
                sect.mark_all_blocks_as_modified();
            }
        } else {
            for i in 0..SECTION_BLOCK_COUNT {
                if block != sect.block_states.get(i) {
                    sect.mark_block_as_modified(i);
                }
            }
        }

        if block.is_air() {
            sect.non_air_count = 0;
        } else {
            sect.non_air_count = SECTION_BLOCK_COUNT as u16;
        }

        sect.block_states.fill(block);
    }

    fn biome(&self, x: usize, y: usize, z: usize) -> BiomeId {
        assert!(
            x < 4 && y < self.section_count() * 4 && z < 4,
            "chunk biome offsets of ({x}, {y}, {z}) are out of bounds"
        );

        self.sections[y / 4].biomes.get(x + z * 4 + y % 4 * 4 * 4)
    }

    fn set_biome(&mut self, x: usize, y: usize, z: usize, biome: BiomeId) -> BiomeId {
        assert!(
            x < 4 && y < self.section_count() * 4 && z < 4,
            "chunk biome offsets of ({x}, {y}, {z}) are out of bounds"
        );

        let old_biome = self.sections[y / 4]
            .biomes
            .set(x + z * 4 + y % 4 * 4 * 4, biome);

        if biome != old_biome {
            self.any_biomes_modified = true;
        }

        old_biome
    }

    fn fill_biomes(&mut self, sect_y: usize, biome: BiomeId) {
        let Some(sect) = self.sections.get_mut(sect_y) else {
            panic!(
                "section index {sect_y} out of bounds for chunk with {} sections",
                self.section_count()
            )
        };

        sect.biomes.fill(biome);

        // TODO: this is set to true unconditionally, but it doesn't have to be.
        self.any_biomes_modified = true;
    }

    fn optimize(&mut self) {
        for sect in self.sections.iter_mut() {
            sect.block_states.optimize();
            sect.biomes.optimize();
        }

        self.cached_init_packet.get_mut().unwrap().shrink_to_fit();
        self.cached_update_packets.shrink_to_fit();
    }
}

fn compact_u64s_len(vals_count: usize, bits_per_val: usize) -> usize {
    let vals_per_u64 = 64 / bits_per_val;
    num::Integer::div_ceil(&vals_count, &vals_per_u64)
}

#[inline]
fn encode_compact_u64s(
    mut w: impl Write,
    mut vals: impl Iterator<Item = u64>,
    bits_per_val: usize,
) -> anyhow::Result<()> {
    debug_assert!(bits_per_val <= 64);

    let vals_per_u64 = 64 / bits_per_val;

    loop {
        let mut n = 0;
        for i in 0..vals_per_u64 {
            match vals.next() {
                Some(val) => {
                    debug_assert!(val < 2_u128.pow(bits_per_val as _) as _);
                    n |= val << (i * bits_per_val);
                }
                None if i > 0 => return n.encode(&mut w),
                None => return Ok(()),
            }
        }
        n.encode(&mut w)?;
    }
}

#[cfg(test)]
mod tests {
    use rand::prelude::*;

    use super::*;
    use crate::config::MockConfig;

    fn check_invariants(sections: &[ChunkSection]) {
        for sect in sections {
            assert_eq!(
                (0..SECTION_BLOCK_COUNT)
                    .filter(|&i| !sect.block_states.get(i).is_air())
                    .count(),
                sect.non_air_count as usize,
                "number of non-air blocks does not match counter"
            );
        }
    }

    fn rand_block_state(rng: &mut (impl Rng + ?Sized)) -> BlockState {
        BlockState::from_raw(rng.gen_range(0..=BlockState::max_raw())).unwrap()
    }

    #[test]
    fn random_block_assignments() {
        let mut rng = thread_rng();

        let height = 512;

        let mut loaded = LoadedChunk::<MockConfig>::new(UnloadedChunk::default(), height / 16, ());
        let mut unloaded = UnloadedChunk::new(height);

        for i in 0..10_000 {
            let state = if i % 250 == 0 {
                [BlockState::AIR, BlockState::CAVE_AIR, BlockState::VOID_AIR]
                    .into_iter()
                    .choose(&mut rng)
                    .unwrap()
            } else {
                rand_block_state(&mut rng)
            };

            let x = rng.gen_range(0..16);
            let y = rng.gen_range(0..height);
            let z = rng.gen_range(0..16);

            loaded.set_block_state(x, y, z, state);
            unloaded.set_block_state(x, y, z, state);
        }

        check_invariants(&loaded.sections);
        check_invariants(&unloaded.sections);

        loaded.optimize();
        unloaded.optimize();

        check_invariants(&loaded.sections);
        check_invariants(&unloaded.sections);

        loaded.fill_block_states(
            rng.gen_range(0..loaded.section_count()),
            rand_block_state(&mut rng),
        );
        unloaded.fill_block_states(
            rng.gen_range(0..loaded.section_count()),
            rand_block_state(&mut rng),
        );

        check_invariants(&loaded.sections);
        check_invariants(&unloaded.sections);
    }
}
