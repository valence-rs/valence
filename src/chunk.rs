//! Chunks and related types.
//!
//! A chunk is a 16x16-block segment of a world with a height determined by the
//! [`Dimension`](crate::dimension::Dimension) of the world.
//!
//! In addition to blocks, chunks also contain [biomes](crate::biome::Biome).
//! Every 4x4x4 segment of blocks in a chunk corresponds to a biome.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io::Write;
use std::iter::FusedIterator;

use bitvec::vec::BitVec;
use paletted_container::{PalettedContainer, PalettedContainerElement};
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};
use valence_nbt::compound;

use crate::biome::BiomeId;
use crate::block::BlockState;
use crate::block_pos::BlockPos;
pub use crate::chunk_pos::ChunkPos;
use crate::config::Config;
use crate::protocol::packets::s2c::play::{
    BlockUpdate, ChunkDataAndUpdateLight, S2cPlayPacket, UpdateSectionBlocks,
};
use crate::protocol::{Encode, VarInt, VarLong};
use crate::util::log2_ceil;

mod paletted_container;

/// A container for all [`LoadedChunk`]s in a [`World`](crate::world::World).
pub struct Chunks<C: Config> {
    chunks: HashMap<ChunkPos, LoadedChunk<C>>,
    dimension_height: i32,
    dimension_min_y: i32,
}

impl<C: Config> Chunks<C> {
    pub(crate) fn new(dimension_height: i32, dimension_min_y: i32) -> Self {
        Self {
            chunks: HashMap::new(),
            dimension_height,
            dimension_min_y,
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
        let dimension_section_count = (self.dimension_height / 16) as usize;
        let loaded = LoadedChunk::new(chunk, dimension_section_count, state);

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

        let mut unloaded = UnloadedChunk {
            sections: loaded.sections.into(),
        };

        for sect in &mut unloaded.sections {
            sect.modified_blocks.fill(0);
            sect.modified_blocks_count = 0;
        }

        Some((unloaded, loaded.state))
    }

    /// Returns the height of all loaded chunks in the world. This returns the
    /// same value as [`Chunk::height`] for all loaded chunks.
    pub fn height(&self) -> usize {
        self.dimension_height as usize
    }

    /// The minimum Y coordinate in world space that chunks in this world can
    /// occupy. This is relevant for [`Chunks::block_state`] and
    /// [`Chunks::set_block_state`].
    pub fn min_y(&self) -> i32 {
        self.dimension_min_y
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
    /// [`Chunk::block_state`].
    pub fn block_state(&self, pos: impl Into<BlockPos>) -> Option<BlockState> {
        let pos = pos.into();
        let chunk_pos = ChunkPos::from(pos);

        let chunk = self.get(chunk_pos)?;

        let y = pos.y.checked_sub(self.dimension_min_y)?.try_into().ok()?;

        if y < chunk.height() {
            Some(chunk.block_state(
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
            if let Some(y) = pos
                .y
                .checked_sub(self.dimension_min_y)
                .and_then(|y| y.try_into().ok())
            {
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

    pub(crate) fn update(&mut self) {
        for (_, chunk) in self.chunks.iter_mut() {
            chunk.update();
        }
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
    /// might be looking for [`Chunks::block_state`] instead.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk.
    fn block_state(&self, x: usize, y: usize, z: usize) -> BlockState;

    /// Sets the block state at the provided offsets in the chunk. The previous
    /// block state at the position is returned.
    ///
    /// **Note**: The arguments to this function are offsets from the minimum
    /// corner of the chunk in _chunk space_ rather than _world space_. You
    /// might be looking for [`Chunks::set_block_state`] instead.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk.
    fn set_block_state(&mut self, x: usize, y: usize, z: usize, block: BlockState) -> BlockState;

    /// Sets every block state in this chunk to the given block state.
    ///
    /// This is semantically equivalent to calling [`set_block_state`] on every
    /// block in the chunk followed by a call to [`optimize`] at the end.
    /// However, this function may be implemented more efficiently.
    ///
    /// [`set_block_state`]: Self::set_block_state
    /// [`optimize`]: Self::optimize
    fn fill_block_states(&mut self, block: BlockState);

    /// Gets the biome at the provided biome offsets in the chunk.
    ///
    /// **Note**: the arguments are **not** block positions. Biomes are 4x4x4
    /// segments of a chunk, so `x` and `z` are in `0..4`.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk.
    fn biome(&self, x: usize, y: usize, z: usize) -> BiomeId;

    /// Sets the biome at the provided offsets in the chunk. The previous
    /// biome at the position is returned.
    ///
    /// **Note**: the arguments are **not** block positions. Biomes are 4x4x4
    /// segments of a chunk, so `x` and `z` are in `0..4`.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk.
    fn set_biome(&mut self, x: usize, y: usize, z: usize, biome: BiomeId) -> BiomeId;

    /// Sets every biome in this chunk to the given biome.
    ///
    /// This is semantically equivalent to calling [`set_biome`] on every
    /// biome in the chunk followed by a call to [`optimize`] at the end.
    /// However, this function may be implemented more efficiently.
    ///
    /// [`set_biome`]: Self::set_biome
    /// [`optimize`]: Self::optimize
    fn fill_biomes(&mut self, biome: BiomeId);

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
    /// [`BiomeId::default()`] with the given height in blocks.
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
    /// [`BlockState::AIR`] and biomes are [`BiomeId::default()`].
    ///
    /// # Panics
    ///
    /// The constraints on `new_height` are the same as [`UnloadedChunk::new`].
    pub fn resize(&mut self, new_height: usize) {
        assert!(
            new_height % 16 == 0 && new_height <= 4064,
            "invalid chunk height of {new_height}"
        );

        let old_height = self.sections.len() * 16;

        if new_height > old_height {
            let additional = (new_height - old_height) / 16;
            self.sections.reserve_exact(additional);
            self.sections
                .resize_with(new_height / 16, ChunkSection::default);
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

    fn block_state(&self, x: usize, y: usize, z: usize) -> BlockState {
        assert!(
            x < 16 && y < self.height() && z < 16,
            "chunk block offsets of ({x}, {y}, {z}) are out of bounds"
        );

        self.sections[y / 16]
            .block_states
            .get(x + z * 16 + y % 16 * 16 * 16)
    }

    fn set_block_state(&mut self, x: usize, y: usize, z: usize, block: BlockState) -> BlockState {
        assert!(
            x < 16 && y < self.height() && z < 16,
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

    fn fill_block_states(&mut self, block: BlockState) {
        for sect in self.sections.iter_mut() {
            // TODO: adjust motion blocking here.

            if block.is_air() {
                sect.non_air_count = 0;
            } else {
                sect.non_air_count = SECTION_BLOCK_COUNT as u16;
            }

            sect.block_states.fill(block);
        }
    }

    fn biome(&self, x: usize, y: usize, z: usize) -> BiomeId {
        assert!(
            x < 4 && y < self.height() / 4 && z < 4,
            "chunk biome offsets of ({x}, {y}, {z}) are out of bounds"
        );

        self.sections[y / 4].biomes.get(x + z * 4 + y % 4 * 4 * 4)
    }

    fn set_biome(&mut self, x: usize, y: usize, z: usize, biome: BiomeId) -> BiomeId {
        assert!(
            x < 4 && y < self.height() / 4 && z < 4,
            "chunk biome offsets of ({x}, {y}, {z}) are out of bounds"
        );

        self.sections[y / 4]
            .biomes
            .set(x + z * 4 + y % 4 * 4 * 4, biome)
    }

    fn fill_biomes(&mut self, biome: BiomeId) {
        for sect in self.sections.iter_mut() {
            sect.biomes.fill(biome);
        }
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
    // TODO block_entities: BTreeMap<u32, BlockEntity>,
    // TODO: motion_blocking_heightmap: Box<[u16; 256]>,
    created_this_tick: bool,
}

/// A 16x16x16 meter volume of blocks, biomes, and light in a chunk.
#[derive(Clone)]
struct ChunkSection {
    block_states: PalettedContainer<BlockState, SECTION_BLOCK_COUNT, { SECTION_BLOCK_COUNT / 2 }>,
    /// Contains a set bit for every block that has been modified in this
    /// section this tick. Ignored in unloaded chunks.
    modified_blocks: [usize; SECTION_BLOCK_COUNT / USIZE_BITS],
    /// The number of blocks that have been modified in this section this tick.
    /// Ignored in unloaded chunks.
    modified_blocks_count: u16,
    /// Number of non-air blocks in this section.
    non_air_count: u16,
    biomes: PalettedContainer<BiomeId, 64, 32>,
}

// [T; 64] Doesn't implement Default so we can't derive :(
impl Default for ChunkSection {
    fn default() -> Self {
        Self {
            block_states: Default::default(),
            modified_blocks: [Default::default(); SECTION_BLOCK_COUNT / USIZE_BITS],
            modified_blocks_count: Default::default(),
            non_air_count: Default::default(),
            biomes: Default::default(),
        }
    }
}

const SECTION_BLOCK_COUNT: usize = 4096;
const USIZE_BITS: usize = usize::BITS as _;

impl PalettedContainerElement for BlockState {
    const DIRECT_BITS: usize = log2_ceil(BlockState::max_raw() as _);
    const MAX_INDIRECT_BITS: usize = 8;
    const MIN_INDIRECT_BITS: usize = 4;

    fn to_bits(self) -> u64 {
        self.to_raw() as _
    }
}

impl PalettedContainerElement for BiomeId {
    const DIRECT_BITS: usize = 6;
    const MAX_INDIRECT_BITS: usize = 4;
    const MIN_INDIRECT_BITS: usize = 0;

    fn to_bits(self) -> u64 {
        self.0 as _
    }
}

impl ChunkSection {
    fn mark_block_as_modified(&mut self, idx: usize) {
        if !self.is_block_modified(idx) {
            self.modified_blocks[idx / USIZE_BITS] |= 1 << (idx % USIZE_BITS);
            self.modified_blocks_count += 1;
        }
    }

    fn mark_all_blocks_as_modified(&mut self) {
        self.modified_blocks.fill(usize::MAX);
        self.modified_blocks_count = SECTION_BLOCK_COUNT as u16;
    }

    fn is_block_modified(&self, idx: usize) -> bool {
        self.modified_blocks[idx / USIZE_BITS] >> (idx % USIZE_BITS) & 1 == 1
    }
}

impl<C: Config> LoadedChunk<C> {
    fn new(mut chunk: UnloadedChunk, dimension_section_count: usize, state: C::ChunkState) -> Self {
        chunk.resize(dimension_section_count * 16);

        Self {
            state,
            sections: chunk.sections.into_boxed_slice(),
            created_this_tick: true,
        }
    }

    /// Returns `true` if this chunk was created during the current tick.
    pub fn created_this_tick(&self) -> bool {
        self.created_this_tick
    }

    /// Gets the chunk data packet for this chunk with the given position.
    pub(crate) fn chunk_data_packet(&self, pos: ChunkPos) -> ChunkDataAndUpdateLight {
        let mut blocks_and_biomes = Vec::new();

        for sect in self.sections.iter() {
            sect.non_air_count.encode(&mut blocks_and_biomes).unwrap();
            sect.block_states.encode(&mut blocks_and_biomes).unwrap();
            sect.biomes.encode(&mut blocks_and_biomes).unwrap();
        }

        ChunkDataAndUpdateLight {
            chunk_x: pos.x,
            chunk_z: pos.z,
            heightmaps: compound! {
                // TODO: placeholder heightmap.
                "MOTION_BLOCKING" => vec![0_i64; 37],
            },
            blocks_and_biomes,
            block_entities: vec![], // TODO
            trust_edges: true,
            // sky_light_mask: bitvec![u64, _; 1; section_count + 2],
            sky_light_mask: BitVec::new(),
            block_light_mask: BitVec::new(),
            empty_sky_light_mask: BitVec::new(),
            empty_block_light_mask: BitVec::new(),
            // sky_light_arrays: vec![[0xff; 2048]; section_count + 2],
            sky_light_arrays: vec![],
            block_light_arrays: vec![],
        }
    }

    /// Returns changes to this chunk as block change packets through the
    /// provided closure.
    pub(crate) fn block_change_packets(
        &self,
        pos: ChunkPos,
        min_y: i32,
        mut push_packet: impl FnMut(S2cPlayPacket),
    ) {
        for (sect_y, sect) in self.sections.iter().enumerate() {
            if sect.modified_blocks_count == 1 {
                let (i, bits) = sect
                    .modified_blocks
                    .iter()
                    .cloned()
                    .enumerate()
                    .find(|(_, n)| *n > 0)
                    .expect("invalid modified count");

                debug_assert_eq!(bits.count_ones(), 1);

                let idx = i * USIZE_BITS + log2_ceil(bits);
                let block = sect.block_states.get(idx);

                let global_x = pos.x * 16 + (idx % 16) as i32;
                let global_y = sect_y as i32 * 16 + (idx / (16 * 16)) as i32 + min_y;
                let global_z = pos.z * 16 + (idx / 16 % 16) as i32;

                push_packet(
                    BlockUpdate {
                        location: BlockPos::new(global_x, global_y, global_z),
                        block_id: VarInt(block.to_raw() as _),
                    }
                    .into(),
                );
            } else if sect.modified_blocks_count > 1 {
                let mut blocks = Vec::with_capacity(sect.modified_blocks_count.into());

                for y in 0..16 {
                    for z in 0..16 {
                        for x in 0..16 {
                            let idx = x as usize + z as usize * 16 + y as usize * 16 * 16;

                            if sect.is_block_modified(idx) {
                                let block_id = sect.block_states.get(idx).to_raw();
                                let compact = (block_id as i64) << 12 | (x << 8 | z << 4 | y);

                                blocks.push(VarLong(compact));
                            }
                        }
                    }
                }

                let chunk_section_position = (pos.x as i64) << 42
                    | (pos.z as i64 & 0x3fffff) << 20
                    | (sect_y as i64 + min_y.div_euclid(16) as i64) & 0xfffff;

                push_packet(
                    UpdateSectionBlocks {
                        chunk_section_position,
                        invert_trust_edges: false,
                        blocks,
                    }
                    .into(),
                );
            }
        }
    }

    fn update(&mut self) {
        for sect in self.sections.iter_mut() {
            if sect.modified_blocks_count > 0 {
                sect.modified_blocks_count = 0;
                sect.modified_blocks.fill(0);
            }
        }
        self.created_this_tick = false;
    }
}

impl<C: Config> Chunk for LoadedChunk<C> {
    fn height(&self) -> usize {
        self.sections.len() * 16
    }

    fn block_state(&self, x: usize, y: usize, z: usize) -> BlockState {
        assert!(
            x < 16 && y < self.height() && z < 16,
            "chunk block offsets of ({x}, {y}, {z}) are out of bounds"
        );

        self.sections[y / 16]
            .block_states
            .get(x + z * 16 + y % 16 * 16 * 16)
    }

    fn set_block_state(&mut self, x: usize, y: usize, z: usize, block: BlockState) -> BlockState {
        assert!(
            x < 16 && y < self.height() && z < 16,
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

            // TODO: adjust MOTION_BLOCKING here.

            sect.mark_block_as_modified(idx);
        }

        old_block
    }

    fn fill_block_states(&mut self, block: BlockState) {
        for sect in self.sections.iter_mut() {
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

            // TODO: adjust motion blocking here.

            if block.is_air() {
                sect.non_air_count = 0;
            } else {
                sect.non_air_count = SECTION_BLOCK_COUNT as u16;
            }

            sect.block_states.fill(block);
        }
    }

    fn biome(&self, x: usize, y: usize, z: usize) -> BiomeId {
        assert!(
            x < 4 && y < self.height() / 4 && z < 4,
            "chunk biome offsets of ({x}, {y}, {z}) are out of bounds"
        );

        self.sections[y / 4].biomes.get(x + z * 4 + y % 4 * 4 * 4)
    }

    fn set_biome(&mut self, x: usize, y: usize, z: usize, biome: BiomeId) -> BiomeId {
        assert!(
            x < 4 && y < self.height() / 4 && z < 4,
            "chunk biome offsets of ({x}, {y}, {z}) are out of bounds"
        );

        self.sections[y / 4]
            .biomes
            .set(x + z * 4 + y % 4 * 4 * 4, biome)
    }

    fn fill_biomes(&mut self, biome: BiomeId) {
        for sect in self.sections.iter_mut() {
            sect.biomes.fill(biome);
        }
    }

    fn optimize(&mut self) {
        for sect in self.sections.iter_mut() {
            sect.block_states.optimize();
            sect.biomes.optimize();
        }
    }
}

/*
fn is_motion_blocking(b: BlockState) -> bool {
    // TODO: use is_solid || is_fluid ?
    !b.is_air()
}
*/

fn compact_u64s_len(vals_count: usize, bits_per_val: usize) -> usize {
    let vals_per_u64 = 64 / bits_per_val;
    num::Integer::div_ceil(&vals_count, &vals_per_u64)
}

#[inline]
fn encode_compact_u64s(
    w: &mut impl Write,
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
                None if i > 0 => return n.encode(w),
                None => return Ok(()),
            }
        }
        n.encode(w)?;
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
                sect.modified_blocks
                    .iter()
                    .map(|bits| bits.count_ones() as u16)
                    .sum::<u16>(),
                sect.modified_blocks_count,
                "number of modified blocks does not match counter"
            );

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

        loaded.fill_block_states(rand_block_state(&mut rng));
        unloaded.fill_block_states(rand_block_state(&mut rng));

        check_invariants(&loaded.sections);
        check_invariants(&unloaded.sections);
    }
}
