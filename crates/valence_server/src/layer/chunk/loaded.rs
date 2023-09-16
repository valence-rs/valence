use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::mem;
use std::sync::atomic::{AtomicU32, Ordering};

use parking_lot::Mutex; // Using nonstandard mutex to avoid poisoning API.
use valence_nbt::{compound, Compound, Value};
use valence_protocol::encode::{PacketWriter, WritePacket};
use valence_protocol::packets::play::chunk_data_s2c::ChunkDataBlockEntity;
use valence_protocol::packets::play::chunk_delta_update_s2c::ChunkDeltaUpdateEntry;
use valence_protocol::packets::play::{
    BlockEntityUpdateS2c, BlockUpdateS2c, ChunkDataS2c, ChunkDeltaUpdateS2c,
};
use valence_protocol::{BlockPos, BlockState, ChunkPos, ChunkSectionPos, Encode};
use valence_registry::biome::BiomeId;
use valence_registry::RegistryIdx;

use super::chunk::{
    bit_width, check_biome_oob, check_block_oob, check_section_oob, BiomeContainer,
    BlockStateContainer, Chunk, SECTION_BLOCK_COUNT,
};
use super::paletted_container::PalettedContainer;
use super::unloaded::{self, UnloadedChunk};
use super::{ChunkLayerInfo, ChunkLayerMessages, LocalMsg};

#[derive(Debug)]
pub struct LoadedChunk {
    /// A count of the clients viewing this chunk. Useful for knowing if it's
    /// necessary to record changes, since no client would be in view to receive
    /// the changes if this were zero.
    viewer_count: AtomicU32,
    /// Block and biome data for the chunk.
    sections: Box<[Section]>,
    /// The block entities in this chunk.
    block_entities: BTreeMap<u32, Compound>,
    /// The set of block entities that have been modified this tick.
    changed_block_entities: BTreeSet<u32>,
    /// If any biomes in this chunk have been modified this tick.
    changed_biomes: bool,
    /// Cached bytes of the chunk initialization packet. The cache is considered
    /// invalidated if empty. This should be cleared whenever the chunk is
    /// modified in an observable way, even if the chunk is not viewed.
    cached_init_packets: Mutex<Vec<u8>>,
}

#[derive(Clone, Default, Debug)]
struct Section {
    block_states: BlockStateContainer,
    biomes: BiomeContainer,
    /// Contains modifications for the update section packet. (Or the regular
    /// block update packet if len == 1).
    section_updates: Vec<ChunkDeltaUpdateEntry>,
}

impl Section {
    fn count_non_air_blocks(&self) -> u16 {
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

impl LoadedChunk {
    pub(crate) fn new(height: u32) -> Self {
        Self {
            viewer_count: AtomicU32::new(0),
            sections: vec![Section::default(); height as usize / 16].into(),
            block_entities: BTreeMap::new(),
            changed_block_entities: BTreeSet::new(),
            changed_biomes: false,
            cached_init_packets: Mutex::new(vec![]),
        }
    }

    /// Sets the content of this chunk to the supplied [`UnloadedChunk`]. The
    /// given unloaded chunk is [resized] to match the height of this loaded
    /// chunk prior to insertion.
    ///
    /// The previous chunk data is returned.
    ///
    /// [resized]: UnloadedChunk::set_height
    pub(crate) fn insert(&mut self, mut chunk: UnloadedChunk) -> UnloadedChunk {
        chunk.set_height(self.height());

        let old_sections = self
            .sections
            .iter_mut()
            .zip(chunk.sections)
            .map(|(sect, other_sect)| {
                sect.section_updates.clear();

                unloaded::Section {
                    block_states: mem::replace(&mut sect.block_states, other_sect.block_states),
                    biomes: mem::replace(&mut sect.biomes, other_sect.biomes),
                }
            })
            .collect();
        let old_block_entities = mem::replace(&mut self.block_entities, chunk.block_entities);
        self.changed_block_entities.clear();
        self.changed_biomes = false;
        self.cached_init_packets.get_mut().clear();
        self.assert_no_changes();

        UnloadedChunk {
            sections: old_sections,
            block_entities: old_block_entities,
        }
    }

    pub(crate) fn remove(&mut self) -> UnloadedChunk {
        let old_sections = self
            .sections
            .iter_mut()
            .map(|sect| {
                sect.section_updates.clear();

                unloaded::Section {
                    block_states: mem::take(&mut sect.block_states),
                    biomes: mem::take(&mut sect.biomes),
                }
            })
            .collect();
        let old_block_entities = mem::take(&mut self.block_entities);
        self.changed_block_entities.clear();
        self.changed_biomes = false;
        self.cached_init_packets.get_mut().clear();

        self.assert_no_changes();

        UnloadedChunk {
            sections: old_sections,
            block_entities: old_block_entities,
        }
    }

    /// Returns the number of clients in view of this chunk.
    pub fn viewer_count(&self) -> u32 {
        self.viewer_count.load(Ordering::Relaxed)
    }

    /// Like [`Self::viewer_count`], but avoids an atomic operation.
    pub fn viewer_count_mut(&mut self) -> u32 {
        *self.viewer_count.get_mut()
    }

    /// Increments the viewer count.
    pub(crate) fn inc_viewer_count(&self) {
        self.viewer_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrements the viewer count.
    #[track_caller]
    pub(crate) fn dec_viewer_count(&self) {
        let old = self.viewer_count.fetch_sub(1, Ordering::Relaxed);
        debug_assert_ne!(old, 0, "viewer count underflow!");
    }

    /// Performs the changes necessary to prepare this chunk for client updates.
    /// - Chunk change messages are written to the layer.
    /// - Recorded changes are cleared.
    pub(crate) fn update_pre_client(
        &mut self,
        pos: ChunkPos,
        info: &ChunkLayerInfo,
        messages: &mut ChunkLayerMessages,
    ) {
        if *self.viewer_count.get_mut() == 0 {
            // Nobody is viewing the chunk, so no need to send any update packets. There
            // also shouldn't be any changes that need to be cleared.
            self.assert_no_changes();

            return;
        }

        // Block states
        for (sect_y, sect) in self.sections.iter_mut().enumerate() {
            match sect.section_updates.as_slice() {
                &[] => {}
                &[entry] => {
                    let global_x = pos.x * 16 + entry.off_x() as i32;
                    let global_y = info.min_y + sect_y as i32 * 16 + entry.off_y() as i32;
                    let global_z = pos.z * 16 + entry.off_z() as i32;

                    messages.send_local_infallible(LocalMsg::PacketAt { pos }, |buf| {
                        let mut writer = PacketWriter::new(buf, info.threshold);

                        writer.write_packet(&BlockUpdateS2c {
                            position: BlockPos::new(global_x, global_y, global_z),
                            block_id: BlockState::from_raw(entry.block_state() as u16).unwrap(),
                        });
                    });
                }
                entries => {
                    let chunk_sect_pos = ChunkSectionPos {
                        x: pos.x,
                        y: sect_y as i32 + info.min_y.div_euclid(16),
                        z: pos.z,
                    };

                    messages.send_local_infallible(LocalMsg::PacketAt { pos }, |buf| {
                        let mut writer = PacketWriter::new(buf, info.threshold);

                        writer.write_packet(&ChunkDeltaUpdateS2c {
                            chunk_sect_pos,
                            blocks: Cow::Borrowed(entries),
                        });
                    });
                }
            }

            sect.section_updates.clear();
        }

        // Block entities
        for &idx in &self.changed_block_entities {
            let Some(nbt) = self.block_entities.get(&idx) else {
                continue;
            };

            let x = idx % 16;
            let z = (idx / 16) % 16;
            let y = idx / 16 / 16;

            let state = self.sections[y as usize / 16]
                .block_states
                .get(idx as usize % SECTION_BLOCK_COUNT);

            let Some(kind) = state.block_entity_kind() else {
                continue;
            };

            let global_x = pos.x * 16 + x as i32;
            let global_y = info.min_y + y as i32;
            let global_z = pos.z * 16 + z as i32;

            messages.send_local_infallible(LocalMsg::PacketAt { pos }, |buf| {
                let mut writer = PacketWriter::new(buf, info.threshold);

                writer.write_packet(&BlockEntityUpdateS2c {
                    position: BlockPos::new(global_x, global_y, global_z),
                    kind,
                    data: Cow::Borrowed(nbt),
                });
            });
        }

        self.changed_block_entities.clear();

        // Biomes
        if self.changed_biomes {
            self.changed_biomes = false;

            messages.send_local_infallible(LocalMsg::ChangeBiome { pos }, |buf| {
                for sect in self.sections.iter() {
                    sect.biomes
                        .encode_mc_format(
                            &mut *buf,
                            |b| b.to_index() as _,
                            0,
                            3,
                            bit_width(info.biome_registry_len - 1),
                        )
                        .expect("paletted container encode should always succeed");
                }
            });
        }

        // All changes should be cleared.
        self.assert_no_changes();
    }

    /// Generates the `MOTION_BLOCKING` heightmap for this chunk, which stores
    /// the height of the highest non motion-blocking block in each column.
    ///
    /// The lowest value of the heightmap is 0, which means that there are no
    /// motion-blocking blocks in the column. In this case, rain will fall
    /// through the void and there will be no rain particles.
    ///
    /// A value of 1 means that rain particles will appear at the lowest
    /// possible height y=-64, but note that blocks cannot be placed at y=-65.
    ///
    /// We take these two special cases into account by adding a value of 2 to
    /// our heightmap if we find a motion-blocking block, since
    /// `self.block_state(x, 0, z)` corresponds to the block at (x, -64, z)
    /// ingame.
    #[allow(clippy::needless_range_loop)]
    fn motion_blocking(&self) -> Vec<Vec<u32>> {
        let mut heightmap: Vec<Vec<u32>> = vec![vec![0; 16]; 16];

        for z in 0..16 {
            for x in 0..16 {
                for y in (0..self.height()).rev() {
                    if self.block_state(x as u32, y, z as u32).blocks_motion() {
                        heightmap[z][x] = y + 2;
                        break;
                    }
                }
            }
        }

        heightmap
    }

    /// Encodes a given heightmap into the correct format of the
    /// `ChunkDataS2c` packet.
    ///
    /// The heightmap values are stored in a long array. Each value is encoded
    /// as a 9-bit unsigned integer, so every long can hold at most seven
    /// values. The long is padded at the left side with a single zero. Since
    /// there are 256 values for 256 columns in a chunk, there will be 36
    /// fully filled longs and one half-filled long with four values. The
    /// remaining three values in the last long are left unused.
    ///
    /// For example, the `WORLD_SURFACE` heightmap in an empty superflat world
    /// is always 4. The first 36 long values will then be
    ///
    /// 0 000000100 000000100 000000100 000000100 000000100 000000100 000000100,
    ///
    /// and the last long will be
    ///
    /// 0 000000000 000000000 000000000 000000100 000000100 000000100 000000100.
    fn encode_heightmap(&self, heightmap: Vec<Vec<u32>>) -> Value {
        let mut encoded: Vec<i64> = vec![0; 37];
        let mut iter = heightmap.into_iter().flatten();

        for entry in encoded.iter_mut() {
            for j in 0..7 {
                match iter.next() {
                    None => break,
                    Some(y) => *entry += i64::from(y) << (9 * j),
                }
            }
        }

        Value::LongArray(encoded)
    }

    /// Writes the packet data needed to initialize this chunk.
    pub(crate) fn write_init_packets(
        &self,
        mut writer: impl WritePacket,
        pos: ChunkPos,
        info: &ChunkLayerInfo,
    ) {
        let mut init_packets = self.cached_init_packets.lock();

        if init_packets.is_empty() {
            let heightmaps = compound! {
                "MOTION_BLOCKING" => self.encode_heightmap(self.motion_blocking()),
                // TODO Implement `WORLD_SURFACE` (or explain why we don't need it)
                // "WORLD_SURFACE" => self.encode_heightmap(self.world_surface()),
            };

            let mut blocks_and_biomes: Vec<u8> = vec![];

            for sect in self.sections.iter() {
                sect.count_non_air_blocks()
                    .encode(&mut blocks_and_biomes)
                    .unwrap();

                sect.block_states
                    .encode_mc_format(
                        &mut blocks_and_biomes,
                        |b| b.to_raw().into(),
                        4,
                        8,
                        bit_width(BlockState::max_raw().into()),
                    )
                    .expect("paletted container encode should always succeed");

                sect.biomes
                    .encode_mc_format(
                        &mut blocks_and_biomes,
                        |b| b.to_index() as _,
                        0,
                        3,
                        bit_width(info.biome_registry_len - 1),
                    )
                    .expect("paletted container encode should always succeed");
            }

            let block_entities: Vec<_> = self
                .block_entities
                .iter()
                .filter_map(|(&idx, nbt)| {
                    let x = idx % 16;
                    let z = idx / 16 % 16;
                    let y = idx / 16 / 16;

                    let kind = self.sections[y as usize / 16]
                        .block_states
                        .get(idx as usize % SECTION_BLOCK_COUNT)
                        .block_entity_kind();

                    kind.map(|kind| ChunkDataBlockEntity {
                        packed_xz: ((x << 4) | z) as i8,
                        y: y as i16 + info.min_y as i16,
                        kind,
                        data: Cow::Borrowed(nbt),
                    })
                })
                .collect();

            PacketWriter::new(&mut init_packets, info.threshold).write_packet(&ChunkDataS2c {
                pos,
                heightmaps: Cow::Owned(heightmaps),
                blocks_and_biomes: &blocks_and_biomes,
                block_entities: Cow::Owned(block_entities),
                sky_light_mask: Cow::Borrowed(&[]),
                block_light_mask: Cow::Borrowed(&[]),
                empty_sky_light_mask: Cow::Borrowed(&[]),
                empty_block_light_mask: Cow::Borrowed(&[]),
                sky_light_arrays: Cow::Borrowed(&[]),
                block_light_arrays: Cow::Borrowed(&[]),
            })
        }

        writer.write_packet_bytes(&init_packets);
    }

    /// Asserts that no changes to this chunk are currently recorded.
    #[track_caller]
    fn assert_no_changes(&self) {
        #[cfg(debug_assertions)]
        {
            assert!(!self.changed_biomes);
            assert!(self.changed_block_entities.is_empty());

            for sect in self.sections.iter() {
                assert!(sect.section_updates.is_empty());
            }
        }
    }
}

impl Chunk for LoadedChunk {
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

        let sect_y = y / 16;
        let sect = &mut self.sections[sect_y as usize];
        let idx = x + z * 16 + y % 16 * 16 * 16;

        let old_block = sect.block_states.set(idx as usize, block);

        if block != old_block {
            self.cached_init_packets.get_mut().clear();

            if *self.viewer_count.get_mut() > 0 {
                sect.section_updates.push(
                    ChunkDeltaUpdateEntry::new()
                        .with_off_x(x as u8)
                        .with_off_y((y % 16) as u8)
                        .with_off_z(z as u8)
                        .with_block_state(block.to_raw().into()),
                );
            }
        }

        old_block
    }

    fn fill_block_state_section(&mut self, sect_y: u32, block: BlockState) {
        check_section_oob(self, sect_y);

        let sect = &mut self.sections[sect_y as usize];

        if let PalettedContainer::Single(b) = &sect.block_states {
            if *b != block {
                self.cached_init_packets.get_mut().clear();

                if *self.viewer_count.get_mut() > 0 {
                    // The whole section is being modified, so any previous modifications would
                    // be overwritten.
                    sect.section_updates.clear();

                    // Push section updates for all the blocks in the section.
                    sect.section_updates.reserve_exact(SECTION_BLOCK_COUNT);
                    for z in 0..16 {
                        for x in 0..16 {
                            for y in 0..16 {
                                sect.section_updates.push(
                                    ChunkDeltaUpdateEntry::new()
                                        .with_off_x(x)
                                        .with_off_y(y)
                                        .with_off_z(z)
                                        .with_block_state(block.to_raw().into()),
                                );
                            }
                        }
                    }
                }
            }
        } else {
            for z in 0..16 {
                for x in 0..16 {
                    for y in 0..16 {
                        let idx = x + z * 16 + (sect_y * 16 + y) * (16 * 16);

                        if block != sect.block_states.get(idx as usize) {
                            self.cached_init_packets.get_mut().clear();

                            if *self.viewer_count.get_mut() > 0 {
                                sect.section_updates.push(
                                    ChunkDeltaUpdateEntry::new()
                                        .with_off_x(x as u8)
                                        .with_off_y(y as u8)
                                        .with_off_z(z as u8)
                                        .with_block_state(block.to_raw().into()),
                                );
                            }
                        }
                    }
                }
            }
        }

        sect.block_states.fill(block);
    }

    fn block_entity(&self, x: u32, y: u32, z: u32) -> Option<&Compound> {
        check_block_oob(self, x, y, z);

        let idx = x + z * 16 + y * 16 * 16;
        self.block_entities.get(&idx)
    }

    fn block_entity_mut(&mut self, x: u32, y: u32, z: u32) -> Option<&mut Compound> {
        check_block_oob(self, x, y, z);

        let idx = x + z * 16 + y * 16 * 16;

        if let Some(be) = self.block_entities.get_mut(&idx) {
            if *self.viewer_count.get_mut() > 0 {
                self.changed_block_entities.insert(idx);
            }
            self.cached_init_packets.get_mut().clear();

            Some(be)
        } else {
            None
        }
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
            Some(nbt) => {
                if *self.viewer_count.get_mut() > 0 {
                    self.changed_block_entities.insert(idx);
                }
                self.cached_init_packets.get_mut().clear();

                self.block_entities.insert(idx, nbt)
            }
            None => {
                let res = self.block_entities.remove(&idx);

                if res.is_some() {
                    self.cached_init_packets.get_mut().clear();
                }

                res
            }
        }
    }

    fn clear_block_entities(&mut self) {
        if self.block_entities.is_empty() {
            return;
        }

        self.cached_init_packets.get_mut().clear();

        if *self.viewer_count.get_mut() > 0 {
            self.changed_block_entities
                .extend(mem::take(&mut self.block_entities).into_keys());
        } else {
            self.block_entities.clear();
        }
    }

    fn biome(&self, x: u32, y: u32, z: u32) -> BiomeId {
        check_biome_oob(self, x, y, z);

        let idx = x + z * 4 + y % 4 * 4 * 4;
        self.sections[y as usize / 4].biomes.get(idx as usize)
    }

    fn set_biome(&mut self, x: u32, y: u32, z: u32, biome: BiomeId) -> BiomeId {
        check_biome_oob(self, x, y, z);

        let idx = x + z * 4 + y % 4 * 4 * 4;
        let old_biome = self.sections[y as usize / 4]
            .biomes
            .set(idx as usize, biome);

        if biome != old_biome {
            self.cached_init_packets.get_mut().clear();

            if *self.viewer_count.get_mut() > 0 {
                self.changed_biomes = true;
            }
        }

        old_biome
    }

    fn fill_biome_section(&mut self, sect_y: u32, biome: BiomeId) {
        check_section_oob(self, sect_y);

        let sect = &mut self.sections[sect_y as usize];

        if let PalettedContainer::Single(b) = &sect.biomes {
            if *b != biome {
                self.cached_init_packets.get_mut().clear();
                self.changed_biomes = *self.viewer_count.get_mut() > 0;
            }
        } else {
            self.cached_init_packets.get_mut().clear();
            self.changed_biomes = *self.viewer_count.get_mut() > 0;
        }

        sect.biomes.fill(biome);
    }

    fn shrink_to_fit(&mut self) {
        self.cached_init_packets.get_mut().shrink_to_fit();

        for sect in self.sections.iter_mut() {
            sect.block_states.shrink_to_fit();
            sect.biomes.shrink_to_fit();
            sect.section_updates.shrink_to_fit();
        }
    }
}

#[cfg(test)]
mod tests {
    use valence_protocol::{ident, CompressionThreshold};

    use super::*;

    #[test]
    fn loaded_chunk_unviewed_no_changes() {
        let mut chunk = LoadedChunk::new(512);

        chunk.set_block(0, 10, 0, BlockState::MAGMA_BLOCK);
        chunk.assert_no_changes();

        chunk.set_biome(0, 0, 0, BiomeId::from_index(5));
        chunk.assert_no_changes();

        chunk.fill_block_states(BlockState::ACACIA_BUTTON);
        chunk.assert_no_changes();

        chunk.fill_biomes(BiomeId::from_index(42));
        chunk.assert_no_changes();
    }

    #[test]
    fn loaded_chunk_changes_clear_packet_cache() {
        #[track_caller]
        fn check<T>(chunk: &mut LoadedChunk, change: impl FnOnce(&mut LoadedChunk) -> T) {
            let info = ChunkLayerInfo {
                dimension_type_name: ident!("whatever").into(),
                height: 512,
                min_y: -16,
                biome_registry_len: 200,
                threshold: CompressionThreshold(-1),
            };

            let mut buf = vec![];
            let mut writer = PacketWriter::new(&mut buf, CompressionThreshold(-1));

            // Rebuild cache.
            chunk.write_init_packets(&mut writer, ChunkPos::new(3, 4), &info);

            // Check that the cache is built.
            assert!(!chunk.cached_init_packets.get_mut().is_empty());

            // Making a change should clear the cache.
            change(chunk);
            assert!(chunk.cached_init_packets.get_mut().is_empty());

            // Rebuild cache again.
            chunk.write_init_packets(&mut writer, ChunkPos::new(3, 4), &info);
            assert!(!chunk.cached_init_packets.get_mut().is_empty());
        }

        let mut chunk = LoadedChunk::new(512);

        check(&mut chunk, |c| {
            c.set_block_state(0, 4, 0, BlockState::ACACIA_WOOD)
        });
        check(&mut chunk, |c| c.set_biome(1, 2, 3, BiomeId::from_index(4)));
        check(&mut chunk, |c| c.fill_biomes(BiomeId::DEFAULT));
        check(&mut chunk, |c| c.fill_block_states(BlockState::WET_SPONGE));
        check(&mut chunk, |c| {
            c.set_block_entity(3, 40, 5, Some(compound! {}))
        });
        check(&mut chunk, |c| {
            c.block_entity_mut(3, 40, 5).unwrap();
        });
        check(&mut chunk, |c| c.set_block_entity(3, 40, 5, None));

        // Old block state is the same as new block state, so the cache should still be
        // intact.
        assert_eq!(
            chunk.set_block_state(0, 0, 0, BlockState::WET_SPONGE),
            BlockState::WET_SPONGE
        );

        assert!(!chunk.cached_init_packets.get_mut().is_empty());
    }
}
