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

use super::paletted_container::PalettedContainer;
use super::{
    check_biome_oob, check_block_oob, BlockEntity, UnloadedChunk, SECTION_BIOME_COUNT,
    SECTION_BLOCK_COUNT,
};
use crate::{BlockEntity, Chunk};

#[derive(Debug)]
pub struct LoadedChunk {
    sections: Box<[Section]>,
    /// The block entities in this chunk.
    block_entities: BTreeMap<u32, BlockEntity>,
    /// The set of block entities that have been modified this tick.
    changed_block_entities: BTreeSet<u32>,
    /// Global compression threshold.
    compression_threshold: Option<u32>,
    /// A cache of packets to send to all clients that are in view of this chunk
    /// at the end of the tick. Cleared at the end of the tick.
    packet_buf: Vec<u8>,
    /// Cached bytes of the chunk data packet. The cache is considered
    /// invalidated if empty. This should be cleared whenever the chunk is
    /// modified in an observable way.
    cached_init_packets: Mutex<Vec<u8>>,
    /// Minecraft entities in this chunk.
    entities: BTreeSet<Entity>,
    /// Minecraft entities that have entered the chunk this tick, paired with
    /// the chunk position in this instance they came from.
    incoming_entities: Vec<(Entity, Option<ChunkPos>)>,
    /// Minecraft entities that have left the chunk this tick, paired with the
    /// chunk position in this instance they arrived at.
    outgoing_entities: Vec<(Entity, Option<ChunkPos>)>,
    /// If any biomes in this chunk have been modified this tick.
    changed_biomes: bool,
    /// If this chunk is in view of any clients. Useful for knowing if it's
    /// necessary to record changes.
    is_viewed: AtomicBool,
}

#[derive(Clone, Default, Debug)]
struct Section {
    block_states: PalettedContainer<BlockState, SECTION_BLOCK_COUNT, { SECTION_BLOCK_COUNT / 2 }>,
    biomes: PalettedContainer<BiomeId, SECTION_BIOME_COUNT, { SECTION_BIOME_COUNT / 2 }>,
    /// Number of non-air blocks in this section.
    non_air_count: u16,
    /// Contains modifications for the update section packet. (Or the regular
    /// block update packet if len == 1).
    section_updates: Vec<VarLong>,
}

impl LoadedChunk {
    pub(crate) fn from_unloaded(chunk: UnloadedChunk, compression_threshold: Option<u32>) -> Self {
        Self {
            sections: chunk
                .sections
                .into_iter()
                .map(|sect| {
                    let non_air_count = sect.count_non_air_blocks();

                    Section {
                        block_states: sect.block_states,
                        biomes: sect.biomes,
                        non_air_count,
                        section_updates: vec![],
                    }
                })
                .collect(),
            block_entities: chunk.block_entities,
            changed_block_entities: BTreeMap::new(),
            compression_threshold,
            packet_buf: vec![],
            cached_init_packets: Mutex::new(vec![]),
            entities: BTreeSet::new(),
            incoming_entities: vec![],
            outgoing_entities: vec![],
            changed_biomes: false,
            is_viewed: AtomicBool::new(false),
        }
    }

    pub fn take(&mut self) -> LoadedChunk {
        
    }

    /// If this chunk was in view of any clients at the end of the previous
    /// tick.
    pub fn is_viewed(&self) -> bool {
        self.is_viewed.load(Ordering::Relaxed)
    }

    /// Same as [`Self::is_viewed`], but avoids atomic operations.
    #[doc(hidden)]
    pub fn is_viewed_mut(&mut self) -> bool {
        *self.is_viewed.get_mut()
    }
}

impl Chunk for LoadedChunk {
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

            if !block.is_air() {
                sect.non_air_count = SECTION_BLOCK_COUNT as u16;
            } else {
                sect.non_air_count = 0;
            }

            sect.block_states.fill(block);
        }
    }

    fn block_entity(&self, x: u32, y: u32, z: u32) -> Option<&BlockEntity> {
        check_block_oob(self, x, y, z);

        let idx = (x + z * 16 + y * 16 * 16) as _;

        self.block_entities.get(&idx)
    }

    fn set_block_entity(
        &mut self,
        x: u32,
        y: u32,
        z: u32,
        block_entity: BlockEntity,
    ) -> Option<BlockEntity> {
        check_block_oob(self, x, y, z);

        let idx = (x + z * 16 + y * 16 * 16) as u32;

        match self.block_entities.entry(idx) {
            Entry::Vacant(ve) => {
                ve.insert(block_entity);
                if *self.is_viewed.get_mut() {
                    self.changed_block_entities.insert(idx);
                }
                self.cached_init_packets.get_mut().clear();
                None
            }
            Entry::Occupied(mut oe) => {
                if oe.get() != block_entity {
                    oe.insert(block_entity);
                }
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

        self.sections[y / 4].biomes.get(x + z * 4 + y % 4 * 4 * 4)
    }

    fn set_biome(&mut self, x: u32, y: u32, z: u32, biome: BiomeId) -> BiomeId {
        check_biome_oob(self, x, y, z);

        let old_biome = self.sections[y / 4]
            .biomes
            .set(x + z * 4 + y % 4 * 4 * 4, biome);

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

                        if block != sect.block_states.get(idx) {
                            self.cached_init_packets.get_mut().clear();

                            let packed = block_bits | (x << 8 | z << 4 | sect_y as u32) as i64;
                            sect.section_updates.push(VarLong(packed));
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

        for sect in &mut self.sections {
            sect.block_states.optimize();
            sect.biomes.optimize();
            sect.section_updates.shrink_to_fit();
        }
    }
}

/// Packets written to chunks will be sent to clients in view of the chunk at
/// the end of the tick.
impl WritePacket for LoadedChunk {
    fn write_packet<P>(&mut self, packet: &P)
    where
        P: Packet + Encode,
    {
        if *self.is_viewed.get_mut() {
            PacketWriter::new(&mut self.packet_buf, self.compression_threshold).write_packet(packet)
        }
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        if *self.is_viewed.get_mut() {
            self.packet_buf.extend_from_slice(bytes);
        }
    }
}
