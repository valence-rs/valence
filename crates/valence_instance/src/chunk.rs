pub mod loaded;
mod paletted_container;
pub mod unloaded;

pub use loaded::LoadedChunk;
pub use unloaded::UnloadedChunk;
use valence_biome::BiomeId;
use valence_block::{BlockEntityKind, BlockState};
use valence_nbt::Compound;

use self::paletted_container::PalettedContainer;

/// Common operations on chunks. Notable implementors are [`LoadedChunk`] and
/// [`UnloadedChunk`].
pub trait Chunk {
    /// Gets the height of this chunk in meters.
    #[inline]
    fn height(&self) -> u32;

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
    fn block_entity(&self, x: u32, y: u32, z: u32) -> Option<&BlockEntity>;

    #[track_caller]
    fn set_block_entity(
        &mut self,
        x: u32,
        y: u32,
        z: u32,
        block_entity: BlockEntity,
    ) -> Option<BlockEntity>;

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

#[derive(Clone, PartialEq, Debug)]
pub struct BlockEntity {
    pub kind: BlockEntityKind,
    pub nbt: Compound,
}

const SECTION_BLOCK_COUNT: usize = 16.pow(3);
const SECTION_BIOME_COUNT: usize = 4.pow(3);

/// The maximum height of a chunk.
pub const MAX_HEIGHT: u32 = todo!();

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
