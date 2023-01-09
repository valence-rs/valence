use bevy_ecs::prelude::*;
use valence_nbt::compound;
use valence_protocol::block::BlockState;
use valence_protocol::packets::s2c::play::{
    BlockUpdate, ChunkDataAndUpdateLightEncode, UpdateSectionBlocksEncode,
};
use valence_protocol::{BlockPos, Encode, LengthPrefixedArray, VarInt, VarLong};

use crate::biome::BiomeId;
use crate::chunk_pos::ChunkPos;
use crate::instance::paletted_container::PalettedContainer;
use crate::instance::Instance;
use crate::math::bit_width;
use crate::packet::{PacketWriter, WritePacket};

/// A chunk is a 16x16-meter segment of a world with a variable height. Chunks
/// primarily contain blocks, biomes, and block entities.
///
/// All chunks in an instance have the same height.
#[derive(Debug)]
pub struct Chunk<const LOADED: bool = false> {
    sections: Vec<Section>,
    /// Cached bytes of the chunk data packet. The cache is considered
    /// invalidated if empty.
    cached_init_packets: Vec<u8>,
    /// If clients should receive the chunk data packet instead of block change
    /// packets on update.
    refresh: bool,
    #[cfg(debug_assertions)]
    uuid: uuid::Uuid,
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

const SECTION_BLOCK_COUNT: usize = 16 * 16 * 16;
const SECTION_BIOME_COUNT: usize = 4 * 4 * 4;

impl Chunk<false> {
    /// Constructs a new chunk containing only [`BlockState::AIR`] and
    /// [`BiomeId::default()`] with the given number of sections. A section is a
    /// 16x16x16 meter volume.
    pub fn new(section_count: usize) -> Self {
        let mut chunk = Self {
            sections: vec![],
            cached_init_packets: vec![],
            refresh: true,
            #[cfg(debug_assertions)]
            uuid: uuid::Uuid::from_u128(rand::random()),
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

    pub(super) fn into_loaded(mut self) -> Chunk<true> {
        debug_assert!(self.refresh);

        Chunk {
            sections: self.sections,
            cached_init_packets: self.cached_init_packets,
            refresh: true,
            #[cfg(debug_assertions)]
            uuid: self.uuid,
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
            cached_init_packets: vec![],
            refresh: true,
            #[cfg(debug_assertions)]
            uuid: uuid::Uuid::from_u128(rand::random()),
        }
    }
}

impl Chunk<true> {
    pub(super) fn into_unloaded(mut self) -> Chunk<false> {
        self.cached_init_packets.clear();

        for sect in &mut self.sections {
            sect.section_updates.clear();
        }

        Chunk {
            sections: self.sections,
            cached_init_packets: self.cached_init_packets,
            refresh: true,
            #[cfg(debug_assertions)]
            uuid: self.uuid,
        }
    }

    pub(super) fn write_update_packets(
        &mut self,
        mut writer: impl WritePacket,
        scratch: &mut Vec<u8>,
        pos: ChunkPos,
        min_y: i32,
        compression_threshold: Option<u32>,
        biome_registry_len: usize,
        filler_sky_light_mask: &[u64],
        filler_sky_light_arrays: &[LengthPrefixedArray<u8, 2048>],
    ) -> anyhow::Result<()> {
        if self.refresh {
            writer.write_bytes(self.get_init_packet_bytes(
                pos,
                scratch,
                compression_threshold,
                biome_registry_len,
                filler_sky_light_mask,
                filler_sky_light_arrays,
            )?)
        } else {
            for (sect_y, sect) in &mut self.sections.iter_mut().enumerate() {
                if sect.section_updates.len() == 1 {
                    let packed = sect.section_updates[0].0 as u64;
                    let offset_z = (packed >> 4) & 0xff;
                    let offset_x = (packed >> 8) & 0xff;
                    let block = packed >> 12;

                    let global_x = pos.x * 16 + offset_x as i32;
                    let global_y = min_y + sect_y as i32 * 16;
                    let global_z = pos.z * 16 + offset_z as i32;

                    writer.write_packet(&BlockUpdate {
                        position: BlockPos::new(global_x, global_y, global_z),
                        block_id: VarInt(block as i32),
                    })?
                } else if sect.section_updates.len() > 1 {
                    let chunk_section_position = (pos.x as i64) << 42
                        | (pos.z as i64 & 0x3fffff) << 20
                        | (sect_y as i64 + min_y.div_euclid(16) as i64) & 0xfffff;

                    writer.write_packet(&UpdateSectionBlocksEncode {
                        chunk_section_position,
                        invert_trust_edges: false,
                        blocks: &sect.section_updates,
                    })?;
                }
            }

            Ok(())
        }
    }

    /// Writes the chunk data packet for this chunk with the given position.
    /// This will initialize the chunk for the client.
    pub(crate) fn write_init_packets(
        &mut self,
        pos: ChunkPos,
        instance: &Instance,
        mut writer: impl WritePacket,
        scratch: &mut Vec<u8>,
    ) -> anyhow::Result<()> {
        #[cfg(debug_assertions)]
        assert_eq!(
            instance.chunk(pos).unwrap().uuid,
            self.uuid,
            "instance and/or position arguments are incorrect"
        );

        writer.write_bytes(self.get_init_packet_bytes(
            pos,
            scratch,
            instance.compression_threshold,
            instance.biome_registry_len,
            &instance.filler_sky_light_mask,
            &instance.filler_sky_light_arrays,
        )?)
    }

    pub fn get_init_packet_bytes(
        &mut self,
        pos: ChunkPos,
        scratch: &mut Vec<u8>,
        compression_threshold: Option<u32>,
        biome_registry_len: usize,
        filler_sky_light_mask: &[u64],
        filler_sky_light_arrays: &[LengthPrefixedArray<u8, 2048>],
    ) -> anyhow::Result<&[u8]> {
        if self.cached_init_packets.is_empty() {
            scratch.clear();

            for sect in &self.sections {
                sect.non_air_count.encode(&mut *scratch).unwrap();

                sect.block_states.encode_mc_format(
                    &mut *scratch,
                    |b| b.to_raw().into(),
                    4,
                    8,
                    bit_width(BlockState::max_raw().into()),
                )?;

                sect.biomes.encode_mc_format(
                    &mut *scratch,
                    |b| b.0.into(),
                    0,
                    3,
                    bit_width(biome_registry_len - 1),
                )?;
            }

            let mut compression_scratch = vec![];

            let mut writer = PacketWriter::new(
                &mut self.cached_init_packets,
                compression_threshold,
                &mut compression_scratch,
            );

            writer.write_packet(&ChunkDataAndUpdateLightEncode {
                chunk_x: pos.x,
                chunk_z: pos.z,
                heightmaps: &compound! {
                    // TODO: MOTION_BLOCKING heightmap
                },
                blocks_and_biomes: scratch,
                block_entities: &[],
                trust_edges: true,
                sky_light_mask: filler_sky_light_mask,
                block_light_mask: &[],
                empty_sky_light_mask: &[],
                empty_block_light_mask: &[],
                sky_light_arrays: filler_sky_light_arrays,
                block_light_arrays: &[],
            })?;
        }

        Ok(&self.cached_init_packets)
    }

    pub(super) fn update_post_client(&mut self) {
        self.refresh = false;

        for sect in &mut self.sections {
            sect.section_updates.clear();
        }
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
    ///
    /// **Note**: The arguments to this function are offsets from the minimum
    /// corner of the chunk in _chunk space_ rather than _world space_.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk. `x` and `z`
    /// must be less than 16 while `y` must be less than `section_count() * 16`.
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
                self.cached_init_packets.clear();
                let compact = (block.to_raw() as i64) << 12 | (x << 8 | z << 4 | sect_y) as i64;
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
                    self.cached_init_packets.clear();

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
                            self.cached_init_packets.clear();
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

    /// Gets the biome at the provided biome offsets in the chunk.
    ///
    /// **Note**: the arguments are **not** block positions. Biomes are 4x4x4
    /// segments of a chunk, so `x` and `z` are in `0..4`.
    ///
    /// # Panics
    ///
    /// Panics if the offsets are outside the bounds of the chunk. `x` and `z`
    /// must be less than 4 while `y` must be less than `section_count() * 4`.
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
    pub fn set_biome(&mut self, x: usize, y: usize, z: usize, biome: BiomeId) -> BiomeId {
        assert!(
            x < 4 && y < self.section_count() * 4 && z < 4,
            "chunk biome offsets of ({x}, {y}, {z}) are out of bounds"
        );

        let old_biome = self.sections[y / 4]
            .biomes
            .set(x + z * 4 + y % 4 * 4 * 4, biome);

        if LOADED && biome != old_biome {
            self.cached_init_packets.clear();
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
    pub fn fill_biomes(&mut self, sect_y: usize, biome: BiomeId) {
        let Some(sect) = self.sections.get_mut(sect_y) else {
            panic!(
                "section index {sect_y} out of bounds for chunk with {} section(s)",
                self.section_count()
            )
        };

        sect.biomes.fill(biome);

        // TODO: this is set unconditionally, but it doesn't have to be.
        self.cached_init_packets.clear();
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
        self.cached_init_packets.shrink_to_fit();

        for sect in &mut self.sections {
            sect.section_updates.shrink_to_fit();
            sect.block_states.optimize();
            sect.biomes.optimize();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::block::BlockState;

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
}
