use valence_nbt::Compound;
use valence_protocol::{BlockPos, BlockState};
use valence_registry::biome::BiomeId;

mod loaded;
mod paletted_container;
mod unloaded;

pub use loaded::LoadedChunk;
pub use unloaded::Chunk;

use super::{BiomePos, Block, BlockRef};

/// Common operations on chunks. Notable implementors are
/// [`LoadedChunk`](super::loaded::LoadedChunk) and
/// [`UnloadedChunk`](super::unloaded::UnloadedChunk).
pub trait ChunkOps {
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
    fn set_block(&mut self, x: u32, y: u32, z: u32, block: impl Into<Block>) -> Block {
        let block = block.into();
        let old_state = self.set_block_state(x, y, z, block.state);

        let old_nbt = if block.nbt.is_none() && block.state.block_entity_kind().is_some() {
            // If the block state is associated with a block entity, make sure there's
            // always some NBT data. Otherwise, the block will appear invisible to clients
            // when loading the chunk.
            self.set_block_entity(x, y, z, Some(Compound::default()))
        } else {
            self.set_block_entity(x, y, z, block.nbt)
        };

        Block {
            state: old_state,
            nbt: old_nbt,
        }
    }

    /*
    /// Sets all the blocks in the entire chunk to the provided block.
    fn fill_blocks(&mut self, block: impl Into<Block>) {
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
    */

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

/// The maximum height of a chunk.
pub const MAX_HEIGHT: u32 = 4096;

pub(super) const SECTION_BLOCK_COUNT: usize = 16 * 16 * 16;
pub(super) const SECTION_BIOME_COUNT: usize = 4 * 4 * 4;

/// Returns the minimum number of bits needed to represent the integer `n`.
pub(super) const fn bit_width(n: usize) -> usize {
    (usize::BITS - n.leading_zeros()) as _
}

#[inline]
pub(super) fn block_offsets(block_pos: BlockPos, min_y: i32, height: i32) -> Option<[u32; 3]> {
    let off_x = block_pos.x.rem_euclid(16);
    let off_z = block_pos.z.rem_euclid(16);
    let off_y = block_pos.y.wrapping_sub(min_y);

    if off_y < height {
        Some([off_x as u32, off_y as u32, off_z as u32])
    } else {
        None
    }
}

#[inline]
pub(super) fn biome_offsets(biome_pos: BiomePos, min_y: i32, height: i32) -> Option<[u32; 3]> {
    let off_x = biome_pos.x.rem_euclid(4);
    let off_z = biome_pos.z.rem_euclid(4);
    let off_y = biome_pos.y.wrapping_sub(min_y / 4);

    if off_y < height / 4 {
        Some([off_x as u32, off_y as u32, off_z as u32])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_get_set() {
        fn check(mut chunk: impl ChunkOps) {
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

        let unloaded = Chunk::with_height(512);
        let loaded = LoadedChunk::new(512);

        check(unloaded);
        check(loaded);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn chunk_debug_oob_0() {
        let mut chunk = Chunk::with_height(512);
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
        let mut chunk = Chunk::with_height(512);
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
        let mut chunk = Chunk::with_height(512);
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
        let mut chunk = Chunk::with_height(512);
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
        let mut chunk = Chunk::with_height(512);
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
