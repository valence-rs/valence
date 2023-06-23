use std::collections::{btree_map, BTreeMap};

use valence_biome::BiomeId;

use super::paletted_container::PalettedContainer;
use super::{
    check_biome_oob, check_block_oob, check_section_oob, BiomeContainer, BlockEntity,
    BlockStateContainer, Chunk, SECTION_BLOCK_COUNT,
};


#[derive(Clone, Default, Debug)]
pub struct UnloadedChunk {
    pub(super) sections: Vec<Section>,
    pub(super) block_entities: BTreeMap<u32, BlockEntity>,
}

#[derive(Clone, Default, Debug)]
pub(super) struct Section {
    pub(super) block_states: BlockStateContainer,
    pub(super) biomes: BiomeContainer,
}

impl Section {
    pub(super) fn count_non_air_blocks(&self) -> u32 {
        let mut count = 0;

        match &self.block_states {
            PalettedContainer::Single(s) => {
                if !s.is_air() {
                    count += SECTION_BLOCK_COUNT as u32;
                }
            }
            PalettedContainer::Indirect(ind) => {
                for i in 0..SECTION_BLOCK_COUNT {
                    if !ind.get(i).is_air() {
                        count += 1;
                    }
                }
            }
            PalettedContainer::Direct(dir) => {
                for s in dir {
                    if !s.is_air() {
                        count += 1;
                    }
                }
            }
        }

        count
    }
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
    /// The new height should be a multiple of 16. Otherwise, the height is
    /// rounded down to the nearest multiple of 16.
    pub fn set_height(&mut self, height: u32) {
        let new_count = height as usize / 16;
        let old_count = self.sections.len();

        if new_count < old_count {
            self.sections.truncate(new_count);
            self.sections.shrink_to_fit();

            let cutoff = SECTION_BLOCK_COUNT as u32 * new_count as u32;
            self.block_entities.retain(|idx, _| *idx < cutoff);
        } else if new_count > old_count {
            let diff = new_count - old_count;
            self.sections.reserve_exact(diff);
            self.sections.extend((0..diff).map(Section::default));
        }
    }
}

impl Chunk for UnloadedChunk {
    type OccupiedBlockEntityEntry<'a> = OccupiedBlockEntityEntry<'a>;

    type VacantBlockEntityEntry<'a> = VacantBlockEntityEntry<'a>;

    fn height(&self) -> u32 {
        self.sections.len() as u32 * 16
    }

    fn block_state(&self, x: u32, y: u32, z: u32) -> BlockState {
        check_block_oob(self, x, y, z);

        self.sections[y / 16]
            .block_states
            .get(x + z * 16 + y % 16 * 16 * 16)
    }

    fn set_block_state(&mut self, x: u32, y: u32, z: u32, block: BlockState) -> BlockState {
        check_block_oob(self, x, y, z);

        self.sections[y / 16]
            .block_states
            .set(x + z * 16 + y % 16 * 16 * 16, block)
    }

    fn fill_block_states(&mut self, block: BlockState) {
        for sect in &mut self.sections {
            sect.block_states.fill(block);
        }
    }

    fn block_entity(&self, x: u32, y: u32, z: u32) -> Option<&BlockEntity> {
        check_block_oob(chunk, x, y, z);

        let idx = (x + z * 16 + y * 16 * 16) as u32;

        self.block_entities.get(&idx)
    }

    fn set_block_entity(
        &mut self,
        x: u32,
        y: u32,
        z: u32,
        block_entity: BlockEntity,
    ) -> Option<BlockEntity> {
        let idx = (x + z * 16 + y * 16 * 16) as u32;

        self.block_entities.insert(idx, block_entity)
    }

    fn clear_block_entities(&mut self) {
        self.block_entities.clear();
    }

    fn biome(&self, x: u32, y: u32, z: u32) -> BiomeId {
        check_biome_oob(self, x, y, z);

        self.sections[y / 4].biomes.get(x + z * 4 + y % 4 * 4 * 4)
    }

    fn set_biome(&mut self, x: u32, y: u32, z: u32, biome: BiomeId) -> BiomeId {
        check_biome_oob(self, x, y, z);

        self.sections[y / 4]
            .biomes
            .set(x + z * 4 + y % 4 * 4 * 4, biome)
    }

    fn fill_biomes(&mut self, biome: BiomeId) {
        for sect in &mut self.sections {
            sect.biomes.fill(biome);
        }
    }

    fn optimize(&mut self) {
        for sect in &mut self.sections {
            sect.block_states.optimize();
            sect.biomes.optimize();
        }
    }
}
