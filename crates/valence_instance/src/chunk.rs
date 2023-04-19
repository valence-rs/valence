use std::borrow::Cow;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex; // Using nonstandard mutex to avoid poisoning API.
use valence_biome::BiomeId;
use valence_block::{BlockEntityKind, BlockState};
use valence_core::block_pos::BlockPos;
use valence_core::chunk_pos::ChunkPos;
use valence_core::packet::encode::{PacketWriter, WritePacket};
use valence_core::packet::s2c::play::chunk_data::ChunkDataBlockEntity;
use valence_core::packet::s2c::play::{
    BlockEntityUpdateS2c, BlockUpdateS2c, ChunkDataS2c, ChunkDeltaUpdateS2c,
};
use valence_core::packet::var_int::VarInt;
use valence_core::packet::var_long::VarLong;
use valence_core::packet::Encode;
use valence_nbt::{compound, Compound};

use crate::paletted_container::PalettedContainer;
use crate::{bit_width, InstanceInfo};

/// A chunk is a 16x16-meter segment of a world with a variable height. Chunks
/// primarily contain blocks, biomes, and block entities.
///
/// All chunks in an instance have the same height.
#[derive(Debug)]
pub struct Chunk<const LOADED: bool = false> {
    sections: Vec<Section>,
    /// Cached bytes of the chunk data packet. The cache is considered
    /// invalidated if empty.
    cached_init_packets: Mutex<Vec<u8>>,
    /// If clients should receive the chunk data packet instead of block change
    /// packets on update.
    refresh: bool,
    /// Tracks if any clients are in view of this (loaded) chunk. Useful for
    /// knowing when a chunk should be unloaded.
    viewed: AtomicBool,
    /// Block entities in this chunk
    block_entities: BTreeMap<u32, BlockEntity>,
    modified_block_entities: BTreeSet<u32>,
}

#[derive(Clone, Default, Debug)]
struct Section {
    block_states: PalettedContainer<BlockState, SECTION_BLOCK_COUNT, { SECTION_BLOCK_COUNT / 2 }>,
    biomes: PalettedContainer<BiomeId, SECTION_BIOME_COUNT, { SECTION_BIOME_COUNT / 2 }>,
    /// Number of non-air blocks in this section. This invariant is maintained
    /// even if `track_changes` is false.
    non_air_count: u16,
    /// Contains modifications for the update section packet. (Or the regular
    /// block update packet if len == 1).
    section_updates: Vec<VarLong>,
}

/// Represents a block with an optional block entity
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Block {
    state: BlockState,
    /// Nbt of the block entity
    nbt: Option<Compound>,
}

impl Block {
    pub const AIR: Self = Self {
        state: BlockState::AIR,
        nbt: None,
    };

    pub fn new(state: BlockState) -> Self {
        Self {
            state,
            nbt: state.block_entity_kind().map(|_| Compound::new()),
        }
    }

    pub fn with_nbt(state: BlockState, nbt: Compound) -> Self {
        Self {
            state,
            nbt: state.block_entity_kind().map(|_| nbt),
        }
    }

    pub const fn state(&self) -> BlockState {
        self.state
    }
}

impl From<BlockState> for Block {
    fn from(value: BlockState) -> Self {
        Self::new(value)
    }
}

impl From<BlockRef<'_>> for Block {
    fn from(BlockRef { state, nbt }: BlockRef<'_>) -> Self {
        Self {
            state,
            nbt: nbt.cloned(),
        }
    }
}

impl From<BlockMut<'_>> for Block {
    fn from(value: BlockMut<'_>) -> Self {
        Self {
            state: value.state,
            nbt: value.nbt().cloned(),
        }
    }
}

impl From<&Block> for Block {
    fn from(value: &Block) -> Self {
        value.clone()
    }
}

impl From<&mut Block> for Block {
    fn from(value: &mut Block) -> Self {
        value.clone()
    }
}

/// Immutable reference to a block in a chunk
#[derive(Clone, Copy, Debug)]
pub struct BlockRef<'a> {
    state: BlockState,
    nbt: Option<&'a Compound>,
}

impl<'a> BlockRef<'a> {
    pub const fn state(&self) -> BlockState {
        self.state
    }

    pub const fn nbt(&self) -> Option<&'a Compound> {
        self.nbt
    }
}

/// Mutable reference to a block in a chunk
#[derive(Debug)]
pub struct BlockMut<'a> {
    state: BlockState,
    /// Entry into the block entity map.
    entry: Entry<'a, u32, BlockEntity>,
    modified: &'a mut BTreeSet<u32>,
}

impl<'a> BlockMut<'a> {
    pub const fn state(&self) -> BlockState {
        self.state
    }

    pub fn nbt(&self) -> Option<&Compound> {
        match &self.entry {
            Entry::Occupied(entry) => Some(&entry.get().nbt),
            Entry::Vacant(_) => None,
        }
    }

    pub fn nbt_mut(&mut self) -> Option<&mut Compound> {
        match &mut self.entry {
            Entry::Occupied(entry) => {
                self.modified.insert(*entry.key());
                Some(&mut entry.get_mut().nbt)
            }
            Entry::Vacant(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockEntity {
    pub kind: BlockEntityKind,
    pub nbt: Compound,
}

impl BlockEntity {
    pub fn new(kind: BlockEntityKind, nbt: Compound) -> Self {
        Self { kind, nbt }
    }
}

const SECTION_BLOCK_COUNT: usize = 16 * 16 * 16;
const SECTION_BIOME_COUNT: usize = 4 * 4 * 4;

impl Chunk<false> {
    /// Constructs a new chunk containing only [`BlockState::AIR`] and
    /// [`BiomeId::default()`] with the given number of sections. A section is a
    /// 16x16x16 meter volume.
    pub fn new(section_count: usize) -> Self {
        let mut chunk = Self {
            sections: vec![],
            cached_init_packets: Mutex::new(vec![]),
            refresh: true,
            viewed: AtomicBool::new(false),
            block_entities: BTreeMap::new(),
            modified_block_entities: BTreeSet::new(),
        };

        chunk.resize(section_count);
        chunk
    }

    /// Changes the section count of the chunk to `new_section_count`.
    ///
    /// The chunk is extended and truncated from the top. New blocks are always
    /// [`BlockState::AIR`] and biomes are [`BiomeId::default()`].
    pub fn resize(&mut self, new_section_count: usize) {
        let old_section_count = self.section_count();

        if new_section_count > old_section_count {
            self.sections
                .reserve_exact(new_section_count - old_section_count);
            self.sections
                .resize_with(new_section_count, Section::default);
        } else {
            self.sections.truncate(new_section_count);
        }
    }

    pub(super) fn into_loaded(self) -> Chunk<true> {
        debug_assert!(self.refresh);
        debug_assert!(self.modified_block_entities.is_empty());

        Chunk {
            sections: self.sections,
            cached_init_packets: self.cached_init_packets,
            refresh: true,
            viewed: AtomicBool::new(false),
            block_entities: self.block_entities,
            modified_block_entities: self.modified_block_entities,
        }
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Clone for Chunk {
    fn clone(&self) -> Self {
        Self {
            sections: self.sections.clone(),
            cached_init_packets: Mutex::new(vec![]),
            refresh: true,
            viewed: AtomicBool::new(false),
            block_entities: self.block_entities.clone(),
            modified_block_entities: BTreeSet::new(),
        }
    }
}

impl Chunk<true> {
    /// Creates an unloaded clone of this loaded chunk.
    pub fn to_unloaded(&self) -> Chunk {
        let sections = self
            .sections
            .iter()
            .map(|sect| {
                Section {
                    block_states: sect.block_states.clone(),
                    biomes: sect.biomes.clone(),
                    non_air_count: 0,
                    section_updates: vec![], // Don't clone the section updates.
                }
            })
            .collect();

        Chunk {
            sections,
            cached_init_packets: Mutex::new(vec![]),
            refresh: true,
            viewed: AtomicBool::new(false),
            block_entities: self.block_entities.clone(),
            modified_block_entities: BTreeSet::new(),
        }
    }

    pub(super) fn clear_viewed(&mut self) {
        *self.viewed.get_mut() = false;
    }

    /// Returns `true` if this chunk was in view of a client at the end of the
    /// previous tick.
    pub fn is_viewed(&self) -> bool {
        self.viewed.load(Ordering::Relaxed)
    }

    /// Like [`Self::is_viewed`], but avoids an atomic load.
    pub fn is_viewed_mut(&mut self) -> bool {
        *self.viewed.get_mut()
    }

    /// Marks this chunk as being seen by a client.
    #[doc(hidden)]
    pub fn mark_viewed(&self) {
        self.viewed.store(true, Ordering::Relaxed);
    }

    pub(super) fn into_unloaded(mut self) -> Chunk<false> {
        self.cached_init_packets.get_mut().clear();

        for sect in &mut self.sections {
            sect.section_updates.clear();
        }
        self.modified_block_entities.clear();

        Chunk {
            sections: self.sections,
            cached_init_packets: self.cached_init_packets,
            refresh: true,
            viewed: AtomicBool::new(false),
            block_entities: self.block_entities,
            modified_block_entities: self.modified_block_entities,
        }
    }

    pub(super) fn write_update_packets(
        &mut self,
        mut writer: impl WritePacket,
        scratch: &mut Vec<u8>,
        pos: ChunkPos,
        info: &InstanceInfo,
    ) {
        if self.refresh {
            self.write_init_packets(info, pos, writer, scratch)
        } else {
            for (sect_y, sect) in &mut self.sections.iter_mut().enumerate() {
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
                            invert_trust_edges: false,
                            blocks: Cow::Borrowed(&sect.section_updates),
                        });
                    }
                }
            }
            for idx in &self.modified_block_entities {
                let Some(block_entity) = self.block_entities.get(idx) else {
                    continue
                };
                let x = idx % 16;
                let z = (idx / 16) % 16;
                let y = idx / 16 / 16;

                let global_x = pos.x * 16 + x as i32;
                let global_y = info.min_y + y as i32;
                let global_z = pos.z * 16 + z as i32;

                writer.write_packet(&BlockEntityUpdateS2c {
                    position: BlockPos::new(global_x, global_y, global_z),
                    kind: VarInt(block_entity.kind as i32),
                    data: Cow::Borrowed(&block_entity.nbt),
                })
            }
        }
    }

    /// Writes the chunk data packet for this chunk with the given position.
    /// This will initialize the chunk for the client.
    #[doc(hidden)]
    pub fn write_init_packets(
        &self,
        info: &InstanceInfo,
        pos: ChunkPos,
        mut writer: impl WritePacket,
        scratch: &mut Vec<u8>,
    ) {
        let mut lck = self.cached_init_packets.lock();

        if lck.is_empty() {
            scratch.clear();

            for sect in &self.sections {
                sect.non_air_count.encode(&mut *scratch).unwrap();

                sect.block_states
                    .encode_mc_format(
                        &mut *scratch,
                        |b| b.to_raw().into(),
                        4,
                        8,
                        bit_width(BlockState::max_raw().into()),
                    )
                    .expect("failed to encode block paletted container");

                sect.biomes
                    .encode_mc_format(
                        &mut *scratch,
                        |b| b.0.into(),
                        0,
                        3,
                        bit_width(info.biome_registry_len - 1),
                    )
                    .expect("failed to encode biome paletted container");
            }

            let mut compression_scratch = vec![];

            let mut writer = PacketWriter::new(
                &mut lck,
                info.compression_threshold,
                &mut compression_scratch,
            );

            let block_entities: Vec<_> = self
                .block_entities
                .iter()
                .map(|(idx, block_entity)| {
                    let x = idx % 16;
                    let z = idx / 16 % 16;
                    let y = (idx / 16 / 16) as i16 + info.min_y as i16;

                    ChunkDataBlockEntity {
                        packed_xz: ((x << 4) | z) as i8,
                        y,
                        kind: VarInt(block_entity.kind as i32),
                        data: Cow::Borrowed(&block_entity.nbt),
                    }
                })
                .collect();

            let heightmaps = compound! {
                // TODO: MOTION_BLOCKING heightmap
            };

            writer.write_packet(&ChunkDataS2c {
                pos,
                heightmaps: Cow::Owned(heightmaps),
                blocks_and_biomes: scratch,
                block_entities: Cow::Borrowed(&block_entities),
                trust_edges: true,
                sky_light_mask: Cow::Borrowed(&info.filler_sky_light_mask),
                block_light_mask: Cow::Borrowed(&[]),
                empty_sky_light_mask: Cow::Borrowed(&[]),
                empty_block_light_mask: Cow::Borrowed(&[]),
                sky_light_arrays: Cow::Borrowed(&info.filler_sky_light_arrays),
                block_light_arrays: Cow::Borrowed(&[]),
            });
        }

        writer.write_packet_bytes(&lck);
    }

    pub(super) fn update_post_client(&mut self) {
        self.refresh = false;

        for sect in &mut self.sections {
            sect.section_updates.clear();
        }
        self.modified_block_entities.clear();
    }
}

impl<const LOADED: bool> Chunk<LOADED> {
    /// Returns the number of sections in this chunk. To get the height of the
    /// chunk in meters, multiply the result by 16.
    pub fn section_count(&self) -> usize {
        self.sections.len()
    }

    /// Gets the block state at the provided offsets in the chunk.
    ///
    /// **Note**: The arguments to this function are offsets from the minimum
    /// corner of the chunk in _chunk space_ rather than _world space_.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk. `x` and `z`
    /// must be less than 16 while `y` must be less than `section_count() * 16`.
    #[track_caller]
    pub fn block_state(&self, x: usize, y: usize, z: usize) -> BlockState {
        assert!(
            x < 16 && y < self.section_count() * 16 && z < 16,
            "chunk block offsets of ({x}, {y}, {z}) are out of bounds"
        );

        self.sections[y / 16]
            .block_states
            .get(x + z * 16 + y % 16 * 16 * 16)
    }

    /// Sets the block state at the provided offsets in the chunk. The previous
    /// block state at the position is returned.
    /// Also, the corresponding block entity is placed.
    ///
    /// **Note**: The arguments to this function are offsets from the minimum
    /// corner of the chunk in _chunk space_ rather than _world space_.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk. `x` and `z`
    /// must be less than 16 while `y` must be less than `section_count() * 16`.
    #[track_caller]
    pub fn set_block_state(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        block: BlockState,
    ) -> BlockState {
        assert!(
            x < 16 && y < self.section_count() * 16 && z < 16,
            "chunk block offsets of ({x}, {y}, {z}) are out of bounds"
        );

        let sect_y = y / 16;
        let sect = &mut self.sections[sect_y];
        let idx = x + z * 16 + y % 16 * 16 * 16;

        let old_block = sect.block_states.set(idx, block);

        if block != old_block {
            // Update non-air count.
            match (block.is_air(), old_block.is_air()) {
                (true, false) => sect.non_air_count -= 1,
                (false, true) => sect.non_air_count += 1,
                _ => {}
            }

            if LOADED && !self.refresh {
                self.cached_init_packets.get_mut().clear();
                let compact = (block.to_raw() as i64) << 12 | (x << 8 | z << 4 | (y % 16)) as i64;
                sect.section_updates.push(VarLong(compact));
            }
        }

        old_block
    }

    /// Sets every block in a section to the given block state.
    ///
    /// This is semantically equivalent to setting every block in the section
    /// with [`set_block_state`]. However, this function may be implemented more
    /// efficiently.
    ///
    /// # Panics
    ///
    /// Panics if `sect_y` is out of bounds. `sect_y` must be less than the
    /// section count.
    ///
    /// [`set_block_state`]: Self::set_block_state
    #[track_caller]
    pub fn fill_block_states(&mut self, sect_y: usize, block: BlockState) {
        let Some(sect) = self.sections.get_mut(sect_y) else {
            panic!(
                "section index {sect_y} out of bounds for chunk with {} sections",
                self.section_count()
            )
        };

        if LOADED && !self.refresh {
            if let PalettedContainer::Single(single) = &sect.block_states {
                if block != *single {
                    self.cached_init_packets.get_mut().clear();

                    // The whole section is being modified, so any previous modifications would be
                    // overwritten.
                    sect.section_updates.clear();

                    // Push section updates for all the blocks in the chunk.
                    sect.section_updates.reserve_exact(SECTION_BLOCK_COUNT);
                    let block_bits = (block.to_raw() as i64) << 12;
                    for z in 0..16 {
                        for x in 0..16 {
                            let packed = block_bits | (x << 8 | z << 4 | sect_y as i64);
                            sect.section_updates.push(VarLong(packed));
                        }
                    }
                }
            } else {
                let block_bits = (block.to_raw() as i64) << 12;
                for z in 0..16 {
                    for x in 0..16 {
                        let idx = x + z * 16 + sect_y * (16 * 16);
                        if block != sect.block_states.get(idx) {
                            self.cached_init_packets.get_mut().clear();
                            let packed = block_bits | (x << 8 | z << 4 | sect_y) as i64;
                            sect.section_updates.push(VarLong(packed));
                        }
                    }
                }
            }
        }

        if !block.is_air() {
            sect.non_air_count = SECTION_BLOCK_COUNT as u16;
        } else {
            sect.non_air_count = 0;
        }

        sect.block_states.fill(block);
    }

    /// Gets a reference to the block entity at the provided offsets in the
    /// chunk.
    ///
    /// **Note**: The arguments to this function are offsets from the minimum
    /// corner of the chunk in _chunk space_ rather than _world space_.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk. `x` and `z`
    /// must be less than 16 while `y` must be less than `section_count() * 16`.
    #[track_caller]
    pub fn block_entity(&self, x: usize, y: usize, z: usize) -> Option<&BlockEntity> {
        assert!(
            x < 16 && y < self.section_count() * 16 && z < 16,
            "chunk block offsets of ({x}, {y}, {z}) are out of bounds"
        );
        let idx = (x + z * 16 + y * 16 * 16) as _;

        self.block_entities.get(&idx)
    }

    /// Sets the block entity at the provided offsets in the chunk.
    /// Returns the block entity that was there before.
    ///
    /// **Note**: The arguments to this function are offsets from the minimum
    /// corner of the chunk in _chunk space_ rather than _world space_.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk. `x` and `z`
    /// must be less than 16 while `y` must be less than `section_count() * 16`.
    #[track_caller]
    pub fn set_block_entity(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        block_entity: BlockEntity,
    ) -> Option<BlockEntity> {
        assert!(
            x < 16 && y < self.section_count() * 16 && z < 16,
            "chunk block offsets of ({x}, {y}, {z}) are out of bounds"
        );
        let idx = (x + z * 16 + y * 16 * 16) as _;
        let old = self.block_entities.insert(idx, block_entity);
        if LOADED && !self.refresh {
            self.modified_block_entities.insert(idx);
            self.cached_init_packets.get_mut().clear();
        }
        old
    }

    /// Returns a mutable reference to the block entity at the provided offsets
    /// in the chunk.
    ///
    /// **Note**: The arguments to this function are offsets from the minimum
    /// corner of the chunk in _chunk space_ rather than _world space_.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk. `x` and `z`
    /// must be less than 16 while `y` must be less than `section_count() * 16`.
    #[track_caller]
    pub fn block_entity_mut(&mut self, x: usize, y: usize, z: usize) -> Option<&mut BlockEntity> {
        assert!(
            x < 16 && y < self.section_count() * 16 && z < 16,
            "chunk block offsets of ({x}, {y}, {z}) are out of bounds"
        );
        let idx = (x + z * 16 + y * 16 * 16) as _;

        let res = self.block_entities.get_mut(&idx);
        if LOADED && res.is_some() && !self.refresh {
            self.modified_block_entities.insert(idx);
            self.cached_init_packets.get_mut().clear();
        }
        self.block_entities.get_mut(&idx)
    }

    /// Sets the block at the provided offsets in the chunk. The previous
    /// block at the position is returned.
    ///
    /// **Note**: The arguments to this function are offsets from the minimum
    /// corner of the chunk in _chunk space_ rather than _world space_.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk. `x` and `z`
    /// must be less than 16 while `y` must be less than `section_count() * 16`.
    #[track_caller]
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: impl Into<Block>) -> Block {
        assert!(
            x < 16 && y < self.section_count() * 16 && z < 16,
            "chunk block offsets of ({x}, {y}, {z}) are out of bounds"
        );

        let Block { state, nbt } = block.into();
        let old_state = {
            let sect_y = y / 16;
            let sect = &mut self.sections[sect_y];
            let idx = x + z * 16 + y % 16 * 16 * 16;

            let old_state = sect.block_states.set(idx, state);

            if state != old_state {
                // Update non-air count.
                match (state.is_air(), old_state.is_air()) {
                    (true, false) => sect.non_air_count -= 1,
                    (false, true) => sect.non_air_count += 1,
                    _ => {}
                }

                if LOADED && !self.refresh {
                    let compact =
                        (state.to_raw() as i64) << 12 | (x << 8 | z << 4 | (y % 16)) as i64;
                    sect.section_updates.push(VarLong(compact));
                }
            }
            old_state
        };

        let idx = (x + z * 16 + y * 16 * 16) as _;
        let old_block_entity = match nbt.and_then(|nbt| {
            state
                .block_entity_kind()
                .map(|kind| BlockEntity { kind, nbt })
        }) {
            Some(block_entity) => self.block_entities.insert(idx, block_entity),
            None => self.block_entities.remove(&idx),
        };
        if LOADED && !self.refresh {
            self.modified_block_entities.insert(idx);
            self.cached_init_packets.get_mut().clear();
        }

        Block {
            state: old_state,
            nbt: old_block_entity.map(|block_entity| block_entity.nbt),
        }
    }

    /// Gets a reference to the block at the provided offsets in the chunk.
    ///
    /// **Note**: The arguments to this function are offsets from the minimum
    /// corner of the chunk in _chunk space_ rather than _world space_.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk. `x` and `z`
    /// must be less than 16 while `y` must be less than `section_count() * 16`.
    #[track_caller]
    pub fn block(&self, x: usize, y: usize, z: usize) -> BlockRef {
        assert!(
            x < 16 && y < self.section_count() * 16 && z < 16,
            "chunk block offsets of ({x}, {y}, {z}) are out of bounds"
        );

        let state = self.sections[y / 16]
            .block_states
            .get(x + z * 16 + y % 16 * 16 * 16);

        let idx = (x + z * 16 + y * 16 * 16) as _;

        let nbt = self
            .block_entities
            .get(&idx)
            .map(|block_entity| &block_entity.nbt);

        BlockRef { state, nbt }
    }

    /// Gets a mutable reference to the block at the provided offsets in the
    /// chunk.
    ///
    /// **Note**: The arguments to this function are offsets from the minimum
    /// corner of the chunk in _chunk space_ rather than _world space_.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk. `x` and `z`
    /// must be less than 16 while `y` must be less than `section_count() * 16`.
    #[track_caller]
    pub fn block_mut(&mut self, x: usize, y: usize, z: usize) -> BlockMut {
        assert!(
            x < 16 && y < self.section_count() * 16 && z < 16,
            "chunk block offsets of ({x}, {y}, {z}) are out of bounds"
        );

        let state = self.sections[y / 16]
            .block_states
            .get(x + z * 16 + y % 16 * 16 * 16);

        let idx = (x + z * 16 + y * 16 * 16) as _;

        let entry = self.block_entities.entry(idx);

        BlockMut {
            state,
            entry,
            modified: &mut self.modified_block_entities,
        }
    }

    /// Gets the biome at the provided biome offsets in the chunk.
    ///
    /// **Note**: the arguments are **not** block positions. Biomes are 4x4x4
    /// segments of a chunk, so `x` and `z` are in `0..4`.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk. `x` and `z`
    /// must be less than 4 while `y` must be less than `section_count() * 4`.
    #[track_caller]
    pub fn biome(&self, x: usize, y: usize, z: usize) -> BiomeId {
        assert!(
            x < 4 && y < self.section_count() * 4 && z < 4,
            "chunk biome offsets of ({x}, {y}, {z}) are out of bounds"
        );

        self.sections[y / 4].biomes.get(x + z * 4 + y % 4 * 4 * 4)
    }

    /// Sets the biome at the provided offsets in the chunk. The previous
    /// biome at the position is returned.
    ///
    /// **Note**: the arguments are **not** block positions. Biomes are 4x4x4
    /// segments of a chunk, so `x` and `z` are in `0..4`.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk. `x` and `z`
    /// must be less than 4 while `y` must be less than `section_count() * 4`.
    #[track_caller]
    pub fn set_biome(&mut self, x: usize, y: usize, z: usize, biome: BiomeId) -> BiomeId {
        assert!(
            x < 4 && y < self.section_count() * 4 && z < 4,
            "chunk biome offsets of ({x}, {y}, {z}) are out of bounds"
        );

        let old_biome = self.sections[y / 4]
            .biomes
            .set(x + z * 4 + y % 4 * 4 * 4, biome);

        if LOADED && biome != old_biome {
            self.cached_init_packets.get_mut().clear();
            self.refresh = true;
        }

        old_biome
    }

    /// Sets every biome in a section to the given block state.
    ///
    /// This is semantically equivalent to setting every biome in the section
    /// with [`set_biome`]. However, this function may be implemented more
    /// efficiently.
    ///
    /// # Panics
    ///
    /// Panics if `sect_y` is out of bounds. `sect_y` must be less than the
    /// section count.
    ///
    /// [`set_biome`]: Self::set_biome
    #[track_caller]
    pub fn fill_biomes(&mut self, sect_y: usize, biome: BiomeId) {
        let Some(sect) = self.sections.get_mut(sect_y) else {
            panic!(
                "section index {sect_y} out of bounds for chunk with {} section(s)",
                self.section_count()
            )
        };

        sect.biomes.fill(biome);

        // TODO: this is set unconditionally, but it doesn't have to be.
        self.cached_init_packets.get_mut().clear();
        self.refresh = true;
    }

    /// Optimizes this chunk to use the minimum amount of memory possible. It
    /// has no observable effect on the contents of the chunk.
    ///
    /// This is a potentially expensive operation. The function is most
    /// effective when a large number of blocks and biomes have changed states
    /// via [`Self::set_block_state`] and [`Self::set_biome`].
    pub fn optimize(&mut self) {
        self.sections.shrink_to_fit();
        self.cached_init_packets.get_mut().shrink_to_fit();

        for sect in &mut self.sections {
            sect.section_updates.shrink_to_fit();
            sect.block_states.optimize();
            sect.biomes.optimize();
        }
    }
}

#[cfg(test)]
mod tests {
    use valence_block::{BlockEntityKind, BlockState};

    use super::*;

    fn check<const LOADED: bool>(chunk: &Chunk<LOADED>, total_expected_change_count: usize) {
        assert!(!chunk.refresh, "chunk should not be refreshed for the test");

        let mut change_count = 0;

        for sect in &chunk.sections {
            assert_eq!(
                (0..SECTION_BLOCK_COUNT)
                    .filter(|&i| !sect.block_states.get(i).is_air())
                    .count(),
                sect.non_air_count as usize,
                "number of non-air blocks does not match counter"
            );

            change_count += sect.section_updates.len();
        }

        assert_eq!(
            change_count, total_expected_change_count,
            "bad change count"
        );
    }

    #[test]
    fn block_state_changes() {
        let mut chunk = Chunk::new(5).into_loaded();
        chunk.refresh = false;

        chunk.set_block_state(0, 0, 0, BlockState::SPONGE);
        check(&chunk, 1);
        chunk.set_block_state(1, 0, 0, BlockState::CAVE_AIR);
        check(&chunk, 2);
        chunk.set_block_state(2, 0, 0, BlockState::MAGMA_BLOCK);
        check(&chunk, 3);
        chunk.set_block_state(2, 0, 0, BlockState::MAGMA_BLOCK);
        check(&chunk, 3);

        chunk.fill_block_states(0, BlockState::AIR);
        check(&chunk, 6);
    }

    #[test]
    fn block_entity_changes() {
        let mut chunk = Chunk::new(5).into_loaded();
        chunk.refresh = false;

        assert!(chunk.block_entity(0, 0, 0).is_none());
        chunk.set_block(0, 0, 0, BlockState::CHEST);
        assert_eq!(
            chunk.block_entity(0, 0, 0),
            Some(&BlockEntity {
                kind: BlockEntityKind::Chest,
                nbt: compound! {}
            })
        );
        chunk.set_block(0, 0, 0, BlockState::STONE);
        assert!(chunk.block_entity(0, 0, 0).is_none());
    }
}
