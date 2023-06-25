use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};

use bevy_ecs::entity::Entity;
use parking_lot::Mutex; // Using nonstandard mutex to avoid poisoning API.
use valence_biome::BiomeId;
use valence_block::BlockState;
use valence_core::chunk_pos::ChunkPos;
use valence_core::protocol::encode::{PacketWriter, WritePacket};
use valence_core::protocol::var_long::VarLong;
use valence_core::protocol::{Encode, Packet};
use valence_nbt::Compound;

use super::paletted_container::PalettedContainer;
use super::{
    check_biome_oob, check_block_oob, unloaded, BiomeContainer, BlockStateContainer, Chunk,
    UnloadedChunk, SECTION_BIOME_COUNT, SECTION_BLOCK_COUNT,
};

#[derive(Debug)]
pub struct LoadedChunk {
    state: ChunkState,
    /// If this chunk is in view of any clients this tick. Useful for knowing if
    /// it's necessary to record changes, since no client would be in view
    /// to receive the change.
    ///
    /// Invariant: `is_viewed` is always `false` while this chunk's state is
    /// [`ChunkState::Added`] or [`ChunkState::Removed`].
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
    /// A cache of packets to send to all clients that are in view of this chunk
    /// at the end of the tick. Cleared at the end of the tick.
    packet_buf: Vec<u8>,
    /// Cached bytes of the chunk initialization packet. The cache is considered
    /// invalidated if empty. This should be cleared whenever the chunk is
    /// modified in an observable way, even if the chunk is not viewed.
    cached_init_packets: Mutex<Vec<u8>>,
    /// Minecraft entities in this chunk.
    entities: BTreeSet<Entity>,
    /// Minecraft entities that have entered the chunk this tick, paired with
    /// the chunk position in this instance they came from. If the position is
    /// `None`, then the entity either moved to a different instance or
    /// despawned.
    incoming_entities: Vec<(Entity, Option<ChunkPos>)>,
    /// Minecraft entities that have left the chunk this tick, paired with the
    /// chunk position in this instance they arrived at. If the position is
    /// `None`, then the entity either moved to a different instance or
    /// despawned.
    outgoing_entities: Vec<(Entity, Option<ChunkPos>)>,
}

/// Describes the current state of a loaded chunk.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ChunkState {
    /// The chunk is newly added this tick. Clients in view of this chunk will
    /// receive the chunk initialization packet.
    Added,
    /// The chunk is marked for removal this tick. Clients in view of this chunk
    /// will receive the chunk deinitialization packet.
    Removed,
    /// The chunk is neither added nor removed.
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

    /// Sets the content of this chunk to the supplied [`UnloadedChunk`] and
    /// sets the state of this chunk to [`ChunkState::Added`]. The given
    /// unloaded chunk is [resized] to match the height of this loaded chunk
    /// prior to insertion.
    ///
    /// The previous chunk data is retuned.
    ///
    /// [resized]: UnloadedChunk::set_height
    pub fn insert(&mut self, mut chunk: UnloadedChunk) -> UnloadedChunk {
        chunk.set_height(self.height());

        self.state = ChunkState::Added;
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

        UnloadedChunk {
            sections: old_sections,
            block_entities: old_block_entities,
        }
    }

    pub fn remove(&mut self) -> UnloadedChunk {
        self.state = ChunkState::Removed;
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

        UnloadedChunk {
            sections: old_sections,
            block_entities: old_block_entities,
        }
    }

    pub fn state(&self) -> ChunkState {
        self.state
    }

    /// If this chunk is potentially in view of any clients.
    pub fn is_viewed(&self) -> bool {
        self.is_viewed.load(Ordering::Relaxed)
    }

    /// Same as [`Self::is_viewed`], but avoids atomic operations.
    #[doc(hidden)]
    pub fn is_viewed_mut(&mut self) -> bool {
        *self.is_viewed.get_mut()
    }

    /// Marks this chunk as being viewed. Not intended for use outside of
    /// `valence_client`.
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
    pub fn get_chunk_init_packets(&self) -> &[u8] {
        todo!()
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

    fn fill_block_states(&mut self, block: BlockState) {
        for (sect_y, sect) in self.sections.iter_mut().enumerate() {
            if let PalettedContainer::Single(b) = &sect.block_states {
                if *b != block {
                    self.cached_init_packets.get_mut().clear();

                    // The whole section is being modified, so any previous modifications would be
                    // overwritten.
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

            sect.block_states.fill(block);
        }
    }

    fn block_entity(&self, x: u32, y: u32, z: u32) -> Option<&Compound> {
        check_block_oob(self, x, y, z);

        let idx = x + z * 16 + y * 16 * 16;
        self.block_entities.get(&idx)
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

        // match self.block_entities.entry(idx) {
        //     Entry::Vacant(ve) => {
        //         ve.insert(block_entity);
        //         self.cached_init_packets.get_mut().clear();
        //         if *self.is_viewed.get_mut() {
        //             self.changed_block_entities.insert(idx);
        //         }
        //         None
        //     }
        //     Entry::Occupied(mut oe) => {
        //         if oe.get() != block_entity {
        //             self.cached_init_packets.get_mut().clear();
        //             if *self.is_viewed.get_mut() {
        //                 self.changed_block_entities.insert(idx);
        //             }
        //         }

        //         match block_entity {
        //             Some(be) => oe.insert(be),
        //             None => oe.remove(),
        //         };
        //     }
        // }

        todo!()
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
            self.changed_biomes = true;
        }

        old_biome
    }

    fn fill_biomes(&mut self, biome: BiomeId) {
        for (sect_y, sect) in self.sections.iter_mut().enumerate() {
            if let PalettedContainer::Single(b) = &sect.biomes {
                if *b != biome {
                    self.cached_init_packets.get_mut().clear();
                    self.changed_biomes = true;
                }
            } else {
                for z in 0..4 {
                    for x in 0..4 {
                        let idx = x + z * 4 + sect_y as u32 * (4 * 4);

                        if biome != sect.biomes.get(idx as usize) {
                            self.cached_init_packets.get_mut().clear();
                        }
                    }
                }
            }

            sect.biomes.fill(biome);
        }
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
