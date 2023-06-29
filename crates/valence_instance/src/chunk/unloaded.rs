use std::collections::BTreeMap;

use valence_biome::BiomeId;
use valence_block::BlockState;
use valence_nbt::Compound;

use super::{
    check_biome_oob, check_block_oob, check_section_oob, BiomeContainer, BlockStateContainer,
    Chunk, MAX_HEIGHT, SECTION_BLOCK_COUNT,
};

#[derive(Clone, Default, Debug)]
pub struct UnloadedChunk {
    pub(super) sections: Vec<Section>,
    pub(super) block_entities: BTreeMap<u32, Compound>,
}

#[derive(Clone, Default, Debug)]
pub(super) struct Section {
    pub(super) block_states: BlockStateContainer,
    pub(super) biomes: BiomeContainer,
}

impl UnloadedChunk {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_height(height: u32) -> Self {
        Self {
            sections: vec![Section::default(); height as usize / 16],
            block_entities: BTreeMap::new(),
        }
    }

    /// Sets the height of this chunk in meters. The chunk is truncated or
    /// extended with [`BlockState::AIR`] and [`BiomeId::default()`] from the
    /// top.
    ///
    /// The new height should be a multiple of 16 and no more than
    /// [`MAX_HEIGHT`]. Otherwise, the height is rounded down to the nearest
    /// valid height.
    pub fn set_height(&mut self, height: u32) {
        let new_count = height.min(MAX_HEIGHT) as usize / 16;
        let old_count = self.sections.len();

        if new_count < old_count {
            self.sections.truncate(new_count);
            self.sections.shrink_to_fit();

            let cutoff = SECTION_BLOCK_COUNT as u32 * new_count as u32;
            self.block_entities.retain(|idx, _| *idx < cutoff);
        } else if new_count > old_count {
            let diff = new_count - old_count;
            self.sections.reserve_exact(diff);
            self.sections.extend((0..diff).map(|_| Section::default()));
        }
    }
}

impl Chunk for UnloadedChunk {
    fn height(&self) -> u32 {
        self.sections.len() as u32 * 16
    }

    fn block_state(&self, x: u32, y: u32, z: u32) -> BlockState {
        check_block_oob(self, x, y, z);

        let idx = x + z * 16 + y % 16 * 16 * 16;
        self.sections[y as usize / 16]
            .block_states
            .get(idx as usize)
    }

    fn set_block_state(&mut self, x: u32, y: u32, z: u32, block: BlockState) -> BlockState {
        check_block_oob(self, x, y, z);

        let idx = x + z * 16 + y % 16 * 16 * 16;
        self.sections[y as usize / 16]
            .block_states
            .set(idx as usize, block)
    }

    fn fill_block_state_section(&mut self, sect_y: u32, block: BlockState) {
        check_section_oob(self, sect_y);

        self.sections[sect_y as usize].block_states.fill(block);
    }

    fn block_entity(&self, x: u32, y: u32, z: u32) -> Option<&Compound> {
        check_block_oob(self, x, y, z);

        let idx = x + z * 16 + y * 16 * 16;
        self.block_entities.get(&idx)
    }

    fn block_entity_mut(&mut self, x: u32, y: u32, z: u32) -> Option<&mut Compound> {
        check_block_oob(self, x, y, z);

        let idx = x + z * 16 + y * 16 * 16;
        self.block_entities.get_mut(&idx)
    }

    fn set_block_entity(
        &mut self,
        x: u32,
        y: u32,
        z: u32,
        block_entity: Option<Compound>,
    ) -> Option<Compound> {
        check_block_oob(self, x, y, z);

        let idx = x + z * 16 + y * 16 * 16;

        match block_entity {
            Some(be) => self.block_entities.insert(idx, be),
            None => self.block_entities.remove(&idx),
        }
    }

    fn clear_block_entities(&mut self) {
        self.block_entities.clear();
    }

    fn biome(&self, x: u32, y: u32, z: u32) -> BiomeId {
        check_biome_oob(self, x, y, z);

        let idx = x + z * 4 + y % 4 * 4 * 4;
        self.sections[y as usize / 4].biomes.get(idx as usize)
    }

    fn set_biome(&mut self, x: u32, y: u32, z: u32, biome: BiomeId) -> BiomeId {
        check_biome_oob(self, x, y, z);

        let idx = x + z * 4 + y % 4 * 4 * 4;
        self.sections[y as usize / 4]
            .biomes
            .set(idx as usize, biome)
    }

    fn fill_biome_section(&mut self, sect_y: u32, biome: BiomeId) {
        check_section_oob(self, sect_y);

        self.sections[sect_y as usize].biomes.fill(biome);
    }

    fn optimize(&mut self) {
        for sect in &mut self.sections {
            sect.block_states.optimize();
            sect.biomes.optimize();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unloaded_chunk_resize_removes_block_entities() {
        let mut chunk = UnloadedChunk::with_height(32);

        assert_eq!(chunk.height(), 32);

        // First block entity is in section 0.
        chunk.set_block_entity(0, 5, 0, Some(Compound::new()));

        // Second block entity is in section 1.
        chunk.set_block_entity(0, 16, 0, Some(Compound::new()));

        // Remove section 0.
        chunk.set_height(16);
        assert_eq!(chunk.height(), 16);

        assert_eq!(chunk.block_entity(0, 5, 0), Some(&Compound::new()));
        assert_eq!(chunk.set_block_entity(0, 5, 0, None), Some(Compound::new()));
        assert!(chunk.block_entities.is_empty());
    }
}
