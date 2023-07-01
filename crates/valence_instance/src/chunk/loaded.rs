use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::*;
use parking_lot::Mutex; // Using nonstandard mutex to avoid poisoning API.
use valence_biome::BiomeId;
use valence_block::BlockState;
use valence_core::block_pos::BlockPos;
use valence_core::chunk_pos::ChunkPos;
use valence_core::despawn::Despawned;
use valence_core::protocol::encode::{PacketWriter, WritePacket};
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::var_long::VarLong;
use valence_core::protocol::{Encode, Packet};
use valence_entity::EntityKind;
use valence_nbt::{compound, Compound};
use valence_registry::RegistryIdx;

use super::paletted_container::PalettedContainer;
use super::{
    bit_width, check_biome_oob, check_block_oob, check_section_oob, unloaded, BiomeContainer,
    BlockStateContainer, Chunk, UnloadedChunk, SECTION_BLOCK_COUNT,
};
use crate::packet::{
    BlockEntityUpdateS2c, BlockUpdateS2c, ChunkBiome, ChunkBiomeDataS2c, ChunkDataBlockEntity,
    ChunkDataS2c, ChunkDeltaUpdateS2c,
};
use crate::{InstanceInfo, UpdateEntityQuery};

#[derive(Debug)]
pub struct LoadedChunk {
    state: ChunkState,
    /// If this chunk is potentially in view of any clients this tick. Useful
    /// for knowing if it's necessary to record changes, since no client
    /// would be in view to receive the changes if this were false.
    ///
    /// Invariant: `is_viewed` is always `false` when this chunk's state not
    /// [`ChunkState::Normal`].
    is_viewed: AtomicBool,
    /// Block and biome data for the chunk.
    sections: Box<[Section]>,
    /// The block entities in this chunk.
    block_entities: BTreeMap<u32, Compound>,
    /// The set of block entities that have been modified this tick.
    changed_block_entities: BTreeSet<u32>,
    /// If any biomes in this chunk have been modified this tick.
    changed_biomes: bool,
    /// The global compression threshold.
    compression_threshold: Option<u32>,
    /// A buffer of packets to send to all clients currently in view of this
    /// chunk at the end of the tick. Clients entering the view of this
    /// chunk this tick should _not_ receive this data.
    ///
    /// Cleared at the end of the tick.
    packet_buf: Vec<u8>,
    /// Cached bytes of the chunk initialization packet. The cache is considered
    /// invalidated if empty. This should be cleared whenever the chunk is
    /// modified in an observable way, even if the chunk is not viewed.
    cached_init_packets: Mutex<Vec<u8>>,
    /// Minecraft entities in this chunk.
    pub(crate) entities: BTreeSet<Entity>,
    /// Minecraft entities that have entered the chunk this tick, paired with
    /// the chunk position in this instance they came from. If the position is
    /// `None`, then the entity either came from an unloaded chunk, a different
    /// instance, or is newly spawned.
    pub(crate) incoming_entities: Vec<(Entity, Option<ChunkPos>)>,
    /// Minecraft entities that have left the chunk this tick, paired with the
    /// chunk position in this instance they arrived at. If the position is
    /// `None`, then the entity either moved to an unloaded chunk, different
    /// instance, or despawned.
    pub(crate) outgoing_entities: Vec<(Entity, Option<ChunkPos>)>,
}

/// Describes the current state of a loaded chunk.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]

pub enum ChunkState {
    /// The chunk is newly inserted this tick.
    Added,
    /// The chunk was `Added` this tick, but then was later removed this tick.
    AddedRemoved,
    /// The chunk was `Normal` in the last tick and has been removed this tick.
    Removed,
    /// The chunk was `Normal` in the last tick and has been overwritten with a
    /// new chunk this tick.
    Overwrite,
    /// The chunk is in none of the other states. This is the common case.
    Normal,
}

#[derive(Clone, Default, Debug)]
struct Section {
    block_states: BlockStateContainer,
    biomes: BiomeContainer,
    /// Contains modifications for the update section packet. (Or the regular
    /// block update packet if len == 1).
    section_updates: Vec<VarLong>,
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
    pub(crate) fn new(height: u32, compression_threshold: Option<u32>) -> Self {
        Self {
            state: ChunkState::Added,
            is_viewed: AtomicBool::new(false),
            sections: vec![Section::default(); height as usize / 16].into(),
            block_entities: BTreeMap::new(),
            changed_block_entities: BTreeSet::new(),
            changed_biomes: false,
            compression_threshold,
            packet_buf: vec![],
            cached_init_packets: Mutex::new(vec![]),
            entities: BTreeSet::new(),
            incoming_entities: vec![],
            outgoing_entities: vec![],
        }
    }

    /// Sets the content of this chunk to the supplied [`UnloadedChunk`]. The
    /// given unloaded chunk is [resized] to match the height of this loaded
    /// chunk prior to insertion.
    ///
    /// The previous chunk data is returned.
    ///
    /// [resized]: UnloadedChunk::set_height
    pub fn insert(&mut self, mut chunk: UnloadedChunk) -> UnloadedChunk {
        chunk.set_height(self.height());

        self.state = match self.state {
            ChunkState::Added => ChunkState::Added,
            ChunkState::AddedRemoved => ChunkState::Added,
            ChunkState::Removed => ChunkState::Overwrite,
            ChunkState::Overwrite => ChunkState::Overwrite,
            ChunkState::Normal => ChunkState::Overwrite,
        };

        *self.is_viewed.get_mut() = false;
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
        self.packet_buf.clear();
        self.cached_init_packets.get_mut().clear();

        self.assert_no_changes();

        UnloadedChunk {
            sections: old_sections,
            block_entities: old_block_entities,
        }
    }

    pub fn remove(&mut self) -> UnloadedChunk {
        self.state = match self.state {
            ChunkState::Added => ChunkState::AddedRemoved,
            ChunkState::AddedRemoved => ChunkState::AddedRemoved,
            ChunkState::Removed => ChunkState::Removed,
            ChunkState::Overwrite => ChunkState::Removed,
            ChunkState::Normal => ChunkState::Removed,
        };

        *self.is_viewed.get_mut() = false;
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
        self.packet_buf.clear();
        self.cached_init_packets.get_mut().clear();

        self.assert_no_changes();

        UnloadedChunk {
            sections: old_sections,
            block_entities: old_block_entities,
        }
    }

    pub fn state(&self) -> ChunkState {
        self.state
    }

    /// All the entities positioned in this chunk.
    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.entities.iter().copied()
    }

    /// If this chunk is potentially in view of any clients.
    pub fn is_viewed(&self) -> bool {
        self.is_viewed.load(Ordering::Relaxed)
    }

    /// Same as [`Self::is_viewed`], but avoids atomic operations. Useful if
    /// mutable access to the chunk was needed anyway.
    #[doc(hidden)]
    pub fn is_viewed_mut(&mut self) -> bool {
        *self.is_viewed.get_mut()
    }

    /// Marks this chunk as being viewed. Not intended for use outside of
    /// `valence_client`. Calling this function at the wrong time might break
    /// things.
    #[doc(hidden)]
    pub fn set_viewed(&self) {
        self.is_viewed.store(true, Ordering::Relaxed);
    }

    /// An immutable view into this chunk's packet buffer.
    #[doc(hidden)]
    pub fn packet_buf(&self) -> &[u8] {
        &self.packet_buf
    }

    #[doc(hidden)]
    pub fn incoming_entities(&self) -> &[(Entity, Option<ChunkPos>)] {
        &self.incoming_entities
    }

    #[doc(hidden)]
    pub fn outgoing_entities(&self) -> &[(Entity, Option<ChunkPos>)] {
        &self.outgoing_entities
    }

    /// Performs the changes necessary to prepare this chunk for client updates.
    /// Notably:
    /// - Chunk and entity update packets are written to this chunk's packet
    ///   buffer.
    /// - Recorded changes are cleared.
    /// - Marks this chunk as unviewed so that clients can mark it as viewed
    ///   during client updates.
    pub(crate) fn update_pre_client(
        &mut self,
        pos: ChunkPos,
        info: &InstanceInfo,
        entity_query: &mut Query<UpdateEntityQuery, (With<EntityKind>, Without<Despawned>)>,
    ) {
        if !*self.is_viewed.get_mut() {
            // Nobody is viewing the chunk, so no need to write to the packet buf. There
            // also shouldn't be any changes that need to be cleared.
            self.assert_no_changes();

            return;
        }

        *self.is_viewed.get_mut() = false;

        debug_assert_eq!(
            self.state,
            ChunkState::Normal,
            "other chunk states should be unviewed"
        );

        let mut writer = PacketWriter::new(&mut self.packet_buf, info.compression_threshold);

        // Block states
        for (sect_y, sect) in self.sections.iter_mut().enumerate() {
            match sect.section_updates.len() {
                0 => {}
                1 => {
                    let packed = sect.section_updates[0].0 as u64;
                    let offset_y = packed & 0b1111;
                    let offset_z = (packed >> 4) & 0b1111;
                    let offset_x = (packed >> 8) & 0b1111;
                    let block = packed >> 12;

                    let global_x = pos.x * 16 + offset_x as i32;
                    let global_y = info.min_y + sect_y as i32 * 16 + offset_y as i32;
                    let global_z = pos.z * 16 + offset_z as i32;

                    writer.write_packet(&BlockUpdateS2c {
                        position: BlockPos::new(global_x, global_y, global_z),
                        block_id: VarInt(block as i32),
                    })
                }
                _ => {
                    let chunk_section_position = (pos.x as i64) << 42
                        | (pos.z as i64 & 0x3fffff) << 20
                        | (sect_y as i64 + info.min_y.div_euclid(16) as i64) & 0xfffff;

                    writer.write_packet(&ChunkDeltaUpdateS2c {
                        chunk_section_position,
                        blocks: Cow::Borrowed(&sect.section_updates),
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

            writer.write_packet(&BlockEntityUpdateS2c {
                position: BlockPos::new(global_x, global_y, global_z),
                kind: VarInt(kind as i32),
                data: Cow::Borrowed(nbt),
            });
        }

        self.changed_block_entities.clear();

        // Biomes
        if self.changed_biomes {
            self.changed_biomes = false;

            // TODO: ChunkBiomeData packet supports updating multiple chunks in a single
            // packet, so it would make more sense to cache the biome data and send it later
            // during client updates.

            let mut biomes: Vec<u8> = vec![];

            for sect in self.sections.iter() {
                sect.biomes
                    .encode_mc_format(
                        &mut biomes,
                        |b| b.to_index() as _,
                        0,
                        3,
                        bit_width(info.biome_registry_len - 1),
                    )
                    .expect("paletted container encode should always succeed");
            }

            self.write_packet(&ChunkBiomeDataS2c {
                chunks: Cow::Borrowed(&[ChunkBiome { pos, data: &biomes }]),
            });
        }

        // Entities
        for &entity in &self.entities {
            let entity = entity_query
                .get_mut(entity)
                .expect("entity in chunk's list of entities should exist");

            let start = self.packet_buf.len();

            let writer = PacketWriter::new(&mut self.packet_buf, self.compression_threshold);
            entity.write_update_packets(writer);

            let end = self.packet_buf.len();

            if let Some(mut range) = entity.packet_byte_range {
                range.0 = start..end;
            }
        }

        // All changes should be cleared.
        self.assert_no_changes();
    }

    pub(crate) fn update_post_client(&mut self) {
        self.packet_buf.clear();
        self.incoming_entities.clear();
        self.outgoing_entities.clear();

        self.state = match self.state {
            ChunkState::Added => ChunkState::Normal,
            ChunkState::AddedRemoved => unreachable!(),
            ChunkState::Removed => unreachable!(),
            ChunkState::Overwrite => ChunkState::Normal,
            ChunkState::Normal => ChunkState::Normal,
        };

        // Changes were already cleared in `write_updates_to_packet_buf`.
        self.assert_no_changes();
    }

    /// Writes the packet data needed to initialize this chunk.
    #[doc(hidden)]
    pub fn write_init_packets(
        &self,
        mut writer: impl WritePacket,
        pos: ChunkPos,
        info: &InstanceInfo,
    ) {
        debug_assert!(
            self.state != ChunkState::Removed && self.state != ChunkState::AddedRemoved,
            "attempt to initialize removed chunk"
        );

        let mut init_packets = self.cached_init_packets.lock();

        if init_packets.is_empty() {
            let heightmaps = compound! {
                // TODO: MOTION_BLOCKING and WORLD_SURFACE heightmaps.
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
                        kind: VarInt(kind as i32),
                        data: Cow::Borrowed(nbt),
                    })
                })
                .collect();

            PacketWriter::new(&mut init_packets, info.compression_threshold).write_packet(
                &ChunkDataS2c {
                    pos,
                    heightmaps: Cow::Owned(heightmaps),
                    blocks_and_biomes: &blocks_and_biomes,
                    block_entities: Cow::Owned(block_entities),
                    sky_light_mask: Cow::Borrowed(&info.sky_light_mask),
                    block_light_mask: Cow::Borrowed(&[]),
                    empty_sky_light_mask: Cow::Borrowed(&[]),
                    empty_block_light_mask: Cow::Borrowed(&[]),
                    sky_light_arrays: Cow::Borrowed(&info.sky_light_arrays),
                    block_light_arrays: Cow::Borrowed(&[]),
                },
            )
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

            if *self.is_viewed.get_mut() {
                let compact = (block.to_raw() as i64) << 12 | (x << 8 | z << 4 | (y % 16)) as i64;
                sect.section_updates.push(VarLong(compact));
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

                if *self.is_viewed.get_mut() {
                    // The whole section is being modified, so any previous modifications would
                    // be overwritten.
                    sect.section_updates.clear();

                    // Push section updates for all the blocks in the section.
                    sect.section_updates.reserve_exact(SECTION_BLOCK_COUNT);
                    let block_bits = (block.to_raw() as i64) << 12;
                    for z in 0..16 {
                        for x in 0..16 {
                            let packed = block_bits | (x << 8 | z << 4 | sect_y as i64);
                            sect.section_updates.push(VarLong(packed));
                        }
                    }
                }
            }
        } else {
            let block_bits = (block.to_raw() as i64) << 12;
            for z in 0..16 {
                for x in 0..16 {
                    let idx = x + z * 16 + sect_y * (16 * 16);
                    if block != sect.block_states.get(idx as usize) {
                        self.cached_init_packets.get_mut().clear();

                        if *self.is_viewed.get_mut() {
                            let packed = block_bits | (x << 8 | z << 4 | sect_y) as i64;
                            sect.section_updates.push(VarLong(packed));
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
            if *self.is_viewed.get_mut() {
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
                if *self.is_viewed.get_mut() {
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

        if *self.is_viewed.get_mut() {
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

            if *self.is_viewed.get_mut() {
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
                self.changed_biomes = *self.is_viewed.get_mut();
            }
        } else {
            self.cached_init_packets.get_mut().clear();
            self.changed_biomes = *self.is_viewed.get_mut();
        }

        sect.biomes.fill(biome);
    }

    fn optimize(&mut self) {
        self.packet_buf.shrink_to_fit();
        self.cached_init_packets.get_mut().shrink_to_fit();
        self.incoming_entities.shrink_to_fit();
        self.outgoing_entities.shrink_to_fit();

        for sect in self.sections.iter_mut() {
            sect.block_states.optimize();
            sect.biomes.optimize();
            sect.section_updates.shrink_to_fit();
        }
    }
}

/// Packets written to chunks will be sent to clients in view of the chunk at
/// the end of the tick.
impl WritePacket for LoadedChunk {
    fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        if *self.is_viewed.get_mut() {
            PacketWriter::new(&mut self.packet_buf, self.compression_threshold)
                .write_packet_fallible(packet)?;
        }

        Ok(())
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        if *self.is_viewed.get_mut() {
            self.packet_buf.extend_from_slice(bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use valence_core::ident;

    use super::*;

    const THRESHOLD: Option<u32> = Some(256);

    #[test]
    fn loaded_chunk_unviewed_no_changes() {
        let mut chunk = LoadedChunk::new(512, THRESHOLD);

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
            let info = InstanceInfo {
                dimension_type_name: ident!("whatever").into(),
                height: 512,
                min_y: -16,
                biome_registry_len: 200,
                compression_threshold: THRESHOLD,
                sky_light_mask: vec![].into(),
                sky_light_arrays: vec![].into(),
            };

            let mut buf = vec![];
            let mut writer = PacketWriter::new(&mut buf, THRESHOLD);

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

        let mut chunk = LoadedChunk::new(512, THRESHOLD);

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
