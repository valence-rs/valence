use std::cmp::Ordering;
use std::collections::BTreeMap;

use valence_nbt::Compound;
use valence_protocol::BlockState;
use valence_registry::biome::BiomeId;

use super::paletted_container::PalettedContainer;
use super::{ChunkOps, MAX_HEIGHT, SECTION_BIOME_COUNT, SECTION_BLOCK_COUNT};

#[derive(Clone, Default, Debug)]
pub struct Chunk {
    pub(super) sections: Vec<Section>,
    pub(super) block_entities: BTreeMap<u32, Compound>,
}

#[derive(Clone, Default, Debug)]
pub(super) struct Section {
    pub(super) block_states:
        PalettedContainer<BlockState, SECTION_BLOCK_COUNT, { SECTION_BLOCK_COUNT / 2 }>,
    pub(super) biomes: PalettedContainer<BiomeId, SECTION_BIOME_COUNT, { SECTION_BIOME_COUNT / 2 }>,
}

impl Section {
    pub(super) fn count_non_air_blocks(&self) -> u16 {
        let mut count = 0;

        match &self.block_states {
            PalettedContainer::Single(s) => {
                if !s.is_air() {
                    count += SECTION_BLOCK_COUNT as u16;
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
                for s in dir.as_ref() {
                    if !s.is_air() {
                        count += 1;
                    }
                }
            }
        }

        count
    }
}

impl Chunk {
    pub const fn new() -> Self {
        Self {
            sections: vec![],
            block_entities: BTreeMap::new(),
        }
    }

    pub fn with_height(height: i32) -> Self {
        Self {
            sections: vec![Section::default(); height.max(0) as usize / 16],
            block_entities: BTreeMap::new(),
        }
    }

    /// Sets the height of this chunk in blocks. The chunk is truncated or
    /// extended with [`BlockState::AIR`] and [`BiomeId::default()`] from the
    /// top.
    ///
    /// The new height should be a multiple of 16 and no more than
    /// [`MAX_HEIGHT`]. Otherwise, the height is rounded down to the nearest
    /// valid height.
    pub fn set_height(&mut self, height: u32) {
        let new_count = height.min(MAX_HEIGHT) as usize / 16;
        let old_count = self.sections.len();

        match new_count.cmp(&old_count) {
            Ordering::Less => {
                self.sections.truncate(new_count);
                self.sections.shrink_to_fit();

                let cutoff = SECTION_BLOCK_COUNT as u32 * new_count as u32;
                self.block_entities.retain(|idx, _| *idx < cutoff);
            }
            Ordering::Equal => {}
            Ordering::Greater => {
                let diff = new_count - old_count;
                self.sections.reserve_exact(diff);
                self.sections.extend((0..diff).map(|_| Section::default()));
            }
        }
    }
}

impl ChunkOps for Chunk {
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

    fn shrink_to_fit(&mut self) {
        for sect in &mut self.sections {
            sect.block_states.shrink_to_fit();
            sect.biomes.shrink_to_fit();
        }
    }
}

#[inline]
#[track_caller]
pub(super) fn check_block_oob(chunk: &impl ChunkOps, x: u32, y: u32, z: u32) {
    assert!(
        x < 16 && y < chunk.height() && z < 16,
        "chunk block offsets of ({x}, {y}, {z}) are out of bounds"
    );
}

#[inline]
#[track_caller]
pub(super) fn check_biome_oob(chunk: &impl ChunkOps, x: u32, y: u32, z: u32) {
    assert!(
        x < 4 && y < chunk.height() / 4 && z < 4,
        "chunk biome offsets of ({x}, {y}, {z}) are out of bounds"
    );
}

#[inline]
#[track_caller]
pub(super) fn check_section_oob(chunk: &impl ChunkOps, sect_y: u32) {
    assert!(
        sect_y < chunk.height() / 16,
        "chunk section offset of {sect_y} is out of bounds"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_resize_removes_block_entities() {
        let mut chunk = Chunk::with_height(32);

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
