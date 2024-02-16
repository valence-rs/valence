use valence_nbt::Compound;
use valence_protocol::BlockState;
use valence_registry::biome::BiomeId;

use super::paletted_container::PalettedContainer;

/// Common operations on chunks. Notable implementors are
/// [`LoadedChunk`](super::loaded::LoadedChunk) and
/// [`UnloadedChunk`](super::unloaded::UnloadedChunk).
pub trait Chunk {
    /// Gets the height of this chunk in meters or blocks.
    fn height(&self) -> u32;

    /// Gets the block at the provided position in this chunk. `x` and `z`
    /// are in the range `0..16` while `y` is in the range `0..height`.
    ///
    /// # Panics
    ///
    /// May panic if the position is out of bounds.
    #[track_caller]
    fn block(&self, x: u32, y: u32, z: u32) -> BlockRef {
        BlockRef {
            state: self.block_state(x, y, z),
            nbt: self.block_entity(x, y, z),
        }
    }

    /// Sets the block at the provided position in this chunk. `x` and `z`
    /// are in the range `0..16` while `y` is in the range `0..height`. The
    /// previous block at the position is returned.
    ///
    /// # Panics
    ///
    /// May panic if the position is out of bounds.
    #[track_caller]
    fn set_block(&mut self, x: u32, y: u32, z: u32, block: impl IntoBlock) -> Block {
        let block = block.into_block();
        let state = self.set_block_state(x, y, z, block.state);
        let nbt = self.set_block_entity(x, y, z, block.nbt);

        Block { state, nbt }
    }

    /// Sets all the blocks in the entire chunk to the provided block.
    fn fill_blocks(&mut self, block: impl IntoBlock) {
        let block = block.into_block();

        self.fill_block_states(block.state);

        if block.nbt.is_some() {
            for x in 0..16 {
                for z in 0..16 {
                    for y in 0..self.height() {
                        self.set_block_entity(x, y, z, block.nbt.clone());
                    }
                }
            }
        } else {
            self.clear_block_entities();
        }
    }

    /// Gets the block state at the provided position in this chunk. `x` and `z`
    /// are in the range `0..16` while `y` is in the range `0..height`.
    ///
    /// # Panics
    ///
    /// May panic if the position is out of bounds.
    #[track_caller]
    fn block_state(&self, x: u32, y: u32, z: u32) -> BlockState;

    /// Sets the block state at the provided position in this chunk. `x` and `z`
    /// are in the range `0..16` while `y` is in the range `0..height`. The
    /// previous block state at the position is returned.
    ///
    /// **NOTE:** This is a low-level function which may break expected
    /// invariants for block entities. Prefer [`Self::set_block`] if performance
    /// is not a concern.
    ///
    /// # Panics
    ///
    /// May panic if the position is out of bounds.
    #[track_caller]
    fn set_block_state(&mut self, x: u32, y: u32, z: u32, block: BlockState) -> BlockState;

    /// Replaces all block states in the entire chunk with the provided block
    /// state.
    ///
    /// **NOTE:** This is a low-level function which may break expected
    /// invariants for block entities. Prefer [`Self::fill_blocks`] instead.
    fn fill_block_states(&mut self, block: BlockState) {
        for sect_y in 0..self.height() / 16 {
            self.fill_block_state_section(sect_y, block);
        }
    }

    /// Replaces all the block states in a section with the provided block
    /// state.
    ///
    /// **NOTE:** This is a low-level function which may break expected
    /// invariants for block entities. Prefer [`Self::set_block`] if performance
    /// is not a concern.
    ///
    /// # Panics
    ///
    /// May panic if the section offset is out of bounds.
    #[track_caller]
    fn fill_block_state_section(&mut self, sect_y: u32, block: BlockState);

    /// Gets the block entity at the provided position in this chunk. `x` and
    /// `z` are in the range `0..16` while `y` is in the range `0..height`.
    ///
    /// # Panics
    ///
    /// May panic if the position is out of bounds.
    #[track_caller]
    fn block_entity(&self, x: u32, y: u32, z: u32) -> Option<&Compound>;

    /// Gets a mutable reference to the block entity at the provided position in
    /// this chunk. `x` and `z` are in the range `0..16` while `y` is in the
    /// range `0..height`.
    ///
    /// # Panics
    ///
    /// May panic if the position is out of bounds.
    #[track_caller]
    fn block_entity_mut(&mut self, x: u32, y: u32, z: u32) -> Option<&mut Compound>;

    /// Sets the block entity at the provided position in this chunk. `x` and
    /// `z` are in the range `0..16` while `y` is in the range `0..height`.
    /// The previous block entity at the position is returned.
    ///
    /// **NOTE:** This is a low-level function which may break expected
    /// invariants for block entities. Prefer [`Self::set_block`] if performance
    /// is not a concern.
    ///
    /// # Panics
    ///
    /// May panic if the position is out of bounds.
    #[track_caller]
    fn set_block_entity(
        &mut self,
        x: u32,
        y: u32,
        z: u32,
        block_entity: Option<Compound>,
    ) -> Option<Compound>;

    /// Removes all block entities from the chunk.
    ///
    /// **NOTE:** This is a low-level function which may break expected
    /// invariants for block entities. Prefer [`Self::set_block`] if performance
    /// is not a concern.
    fn clear_block_entities(&mut self);

    /// Gets the biome at the provided position in this chunk. `x` and `z` are
    /// in the range `0..4` while `y` is in the range `0..height / 4`.
    ///
    /// Note that biomes are 4x4x4 segments of a chunk, so the xyz arguments to
    /// this method differ from those to [`Self::block_state`] and
    /// [`Self::block_entity`].
    ///
    /// # Panics
    ///
    /// May panic if the position is out of bounds.
    #[track_caller]
    fn biome(&self, x: u32, y: u32, z: u32) -> BiomeId;

    /// Sets the biome at the provided position in this chunk. The Previous
    /// biome at the position is returned. `x` and `z` are in the range `0..4`
    /// while `y` is in the range `0..height / 4`.
    ///
    /// Note that biomes are 4x4x4 segments of a chunk, so the xyz arguments to
    /// this method differ from those to [`Self::block_state`] and
    /// [`Self::block_entity`].
    ///
    /// # Panics
    ///
    /// May panic if the position is out of bounds.
    #[track_caller]
    fn set_biome(&mut self, x: u32, y: u32, z: u32, biome: BiomeId) -> BiomeId;

    /// Sets all the biomes in the entire chunk to the provided biome.
    fn fill_biomes(&mut self, biome: BiomeId) {
        for sect_y in 0..self.height() / 16 {
            self.fill_biome_section(sect_y, biome);
        }
    }

    /// Replaces all the biomes in a section with the provided biome.
    ///
    /// # Panics
    ///
    /// May panic if the section offset is out of bounds.
    #[track_caller]
    fn fill_biome_section(&mut self, sect_y: u32, biome: BiomeId);

    /// Sets all blocks and biomes in this chunk to the default values. The
    /// height of the chunk is not modified.
    fn clear(&mut self) {
        self.fill_block_states(BlockState::AIR);
        self.fill_biomes(BiomeId::default());
        self.clear_block_entities();
    }

    /// Attempts to optimize this chunk by reducing its memory usage or other
    /// characteristics. This may be a relatively expensive operation.
    ///
    /// This method must not alter the semantics of the chunk in any observable
    /// way.
    fn shrink_to_fit(&mut self);
}

/// Represents a complete block, which is a pair of block state and optional NBT
/// data for the block entity.
#[derive(Clone, PartialEq, Default, Debug)]
pub struct Block {
    pub state: BlockState,
    pub nbt: Option<Compound>,
}

impl Block {
    pub const fn new(state: BlockState, nbt: Option<Compound>) -> Self {
        Self { state, nbt }
    }
}

/// Like [`Block`], but immutably referenced.
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct BlockRef<'a> {
    pub state: BlockState,
    pub nbt: Option<&'a Compound>,
}

impl<'a> BlockRef<'a> {
    pub const fn new(state: BlockState, nbt: Option<&'a Compound>) -> Self {
        Self { state, nbt }
    }
}

pub trait IntoBlock {
    // TODO: parameterize this with block registry ref?
    fn into_block(self) -> Block;
}

impl IntoBlock for Block {
    fn into_block(self) -> Block {
        self
    }
}

impl<'a> IntoBlock for BlockRef<'a> {
    fn into_block(self) -> Block {
        Block {
            state: self.state,
            nbt: self.nbt.cloned(),
        }
    }
}

/// This will initialize the block with a new empty compound if the block state
/// is associated with a block entity.
impl IntoBlock for BlockState {
    fn into_block(self) -> Block {
        Block {
            state: self,
            nbt: self.block_entity_kind().map(|_| Compound::new()),
        }
    }
}

pub(super) const SECTION_BLOCK_COUNT: usize = 16 * 16 * 16;
pub(super) const SECTION_BIOME_COUNT: usize = 4 * 4 * 4;

/// The maximum height of a chunk.
pub const MAX_HEIGHT: u32 = 4096;

pub(super) type BlockStateContainer =
    PalettedContainer<BlockState, SECTION_BLOCK_COUNT, { SECTION_BLOCK_COUNT / 2 }>;

pub(super) type BiomeContainer =
    PalettedContainer<BiomeId, SECTION_BIOME_COUNT, { SECTION_BIOME_COUNT / 2 }>;

#[inline]
#[track_caller]
pub(super) fn check_block_oob(chunk: &impl Chunk, x: u32, y: u32, z: u32) {
    assert!(
        x < 16 && y < chunk.height() && z < 16,
        "chunk block offsets of ({x}, {y}, {z}) are out of bounds"
    );
}

#[inline]
#[track_caller]
pub(super) fn check_biome_oob(chunk: &impl Chunk, x: u32, y: u32, z: u32) {
    assert!(
        x < 4 && y < chunk.height() / 4 && z < 4,
        "chunk biome offsets of ({x}, {y}, {z}) are out of bounds"
    );
}

#[inline]
#[track_caller]
pub(super) fn check_section_oob(chunk: &impl Chunk, sect_y: u32) {
    assert!(
        sect_y < chunk.height() / 16,
        "chunk section offset of {sect_y} is out of bounds"
    );
}

/// Returns the minimum number of bits needed to represent the integer `n`.
pub(super) const fn bit_width(n: usize) -> usize {
    (usize::BITS - n.leading_zeros()) as _
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::chunk::{LoadedChunk, UnloadedChunk};

    #[test]
    fn chunk_get_set() {
        fn check(mut chunk: impl Chunk) {
            assert_eq!(
                chunk.set_block_state(1, 2, 3, BlockState::CHAIN),
                BlockState::AIR
            );
            assert_eq!(
                chunk.set_block_state(1, 2, 3, BlockState::AIR),
                BlockState::CHAIN
            );

            assert_eq!(chunk.set_block_entity(1, 2, 3, Some(Compound::new())), None);
            assert_eq!(chunk.set_block_entity(1, 2, 3, None), Some(Compound::new()));
        }

        let unloaded = UnloadedChunk::with_height(512);
        let loaded = LoadedChunk::new(512);

        check(unloaded);
        check(loaded);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn chunk_debug_oob_0() {
        let mut chunk = UnloadedChunk::with_height(512);
        chunk.set_block_state(0, 0, 16, BlockState::AIR);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn chunk_debug_oob_1() {
        let mut chunk = LoadedChunk::new(512);
        chunk.set_block_state(0, 0, 16, BlockState::AIR);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn chunk_debug_oob_2() {
        let mut chunk = UnloadedChunk::with_height(512);
        chunk.set_block_entity(0, 0, 16, None);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn chunk_debug_oob_3() {
        let mut chunk = LoadedChunk::new(512);
        chunk.set_block_entity(0, 0, 16, None);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn chunk_debug_oob_4() {
        let mut chunk = UnloadedChunk::with_height(512);
        chunk.set_biome(0, 0, 4, BiomeId::DEFAULT);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn chunk_debug_oob_5() {
        let mut chunk = LoadedChunk::new(512);
        chunk.set_biome(0, 0, 4, BiomeId::DEFAULT);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn chunk_debug_oob_6() {
        let mut chunk = UnloadedChunk::with_height(512);
        chunk.fill_block_state_section(chunk.height() / 16, BlockState::AIR);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn chunk_debug_oob_7() {
        let mut chunk = LoadedChunk::new(512);
        chunk.fill_block_state_section(chunk.height() / 16, BlockState::AIR);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn chunk_debug_oob_8() {
        let mut chunk = UnloadedChunk::with_height(512);
        chunk.fill_biome_section(chunk.height() / 16, BiomeId::DEFAULT);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn chunk_debug_oob_9() {
        let mut chunk = LoadedChunk::new(512);
        chunk.fill_biome_section(chunk.height() / 16, BiomeId::DEFAULT);
    }
}
