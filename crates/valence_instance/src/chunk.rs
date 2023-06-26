pub mod loaded;
mod paletted_container;
pub mod unloaded;

pub use loaded::LoadedChunk;
pub use unloaded::UnloadedChunk;
use valence_biome::BiomeId;
use valence_block::BlockState;
use valence_nbt::Compound;

use self::paletted_container::PalettedContainer;

/// Common operations on chunks. Notable implementors are [`LoadedChunk`] and
/// [`UnloadedChunk`].
pub trait Chunk {
    /// Gets the height of this chunk in meters.
    fn height(&self) -> u32;

    #[track_caller]
    fn block(&self, x: u32, y: u32, z: u32) -> BlockRef {
        BlockRef {
            state: self.block_state(x, y, z),
            nbt: self.block_entity(x, y, z),
        }
    }

    #[track_caller]
    fn set_block(&mut self, x: u32, y: u32, z: u32, block: impl IntoBlock) -> Block {
        let block = block.into_block();
        let state = self.set_block_state(x, y, z, block.state);
        let nbt = self.set_block_entity(x, y, z, block.nbt);

        Block { state, nbt }
    }

    /// Gets the block state at the provided position in this chunk. `x` and `z`
    /// are in the range `0..16` while `y` is in the range `0..height`.
    ///
    /// # Panics
    ///
    /// May panic if the position is out of bounds.
    #[track_caller]
    fn block_state(&self, x: u32, y: u32, z: u32) -> BlockState;

    /// Sets the block state at the provided position in this chunk. The
    /// previous block state at the position is returned. `x` and `z`
    /// are in the range `0..16` while `y` is in the range `0..height`.
    ///
    /// # Panics
    ///
    /// May panic if the position is out of bounds.
    #[track_caller]
    fn set_block_state(&mut self, x: u32, y: u32, z: u32, block: BlockState) -> BlockState;

    fn fill_block_states(&mut self, block: BlockState);

    #[track_caller]
    fn block_entity(&self, x: u32, y: u32, z: u32) -> Option<&Compound>;

    #[track_caller]
    fn set_block_entity(
        &mut self,
        x: u32,
        y: u32,
        z: u32,
        block_entity: Option<Compound>,
    ) -> Option<Compound>;

    /// Removes all block entities from the chunk.
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

    fn fill_biomes(&mut self, biome: BiomeId);

    /// Sets all blocks and biomes in this chunk to the default values. The
    /// height of the chunk is not modified.
    fn clear(&mut self) {
        self.fill_block_states(BlockState::AIR);
        self.fill_biomes(BiomeId::default());
        self.clear_block_entities();
    }

    fn optimize(&mut self);
}

/// Represents a block with optional NBT data.
#[derive(Clone, PartialEq, Default, Debug)]
pub struct Block {
    pub state: BlockState,
    pub nbt: Option<Compound>,
}

/// Referenced variant of [`Block`].
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct BlockRef<'a> {
    pub state: BlockState,
    pub nbt: Option<&'a Compound>,
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

impl IntoBlock for BlockState {
    fn into_block(self) -> Block {
        Block {
            state: self,
            nbt: self.block_entity_kind().map(|_| Compound::new()),
        }
    }
}

const SECTION_BLOCK_COUNT: usize = 16 * 16 * 16;
const SECTION_BIOME_COUNT: usize = 4 * 4 * 4;

/// The maximum height of a chunk.
pub const MAX_HEIGHT: u32 = 4096;

type BlockStateContainer =
    PalettedContainer<BlockState, SECTION_BLOCK_COUNT, { SECTION_BLOCK_COUNT / 2 }>;

type BiomeContainer = PalettedContainer<BiomeId, SECTION_BIOME_COUNT, { SECTION_BIOME_COUNT / 2 }>;

#[inline]
#[track_caller]
fn check_block_oob(chunk: &impl Chunk, x: u32, y: u32, z: u32) {
    assert!(
        x < 16 && y < chunk.height() && z < 16,
        "chunk block offsets of ({x}, {y}, {z}) are out of bounds"
    );
}

#[inline]
#[track_caller]
fn check_biome_oob(chunk: &impl Chunk, x: u32, y: u32, z: u32) {
    assert!(
        x < 4 && y < chunk.height() / 4 && z < 4,
        "chunk biome offsets of ({x}, {y}, {z}) are out of bounds"
    );
}

/// Returns the minimum number of bits needed to represent the integer `n`.
const fn bit_width(n: usize) -> usize {
    (usize::BITS - n.leading_zeros()) as _
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let loaded = LoadedChunk::new(512, None);

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
        let mut chunk = LoadedChunk::new(512, None);
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
        let mut chunk = LoadedChunk::new(512, None);
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
        let mut chunk = LoadedChunk::new(512, None);
        chunk.set_biome(0, 0, 4, BiomeId::DEFAULT);
    }
}
