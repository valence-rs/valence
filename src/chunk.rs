// TODO: https://github.com/rust-lang/rust/issues/88581 for div_ceil

use std::collections::HashMap;
use std::io::Write;
use std::iter::FusedIterator;

use bitvec::vec::BitVec;
use num::Integer;
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::block::BlockState;
use crate::protocol::packets::play::s2c::{
    BlockUpdate, LevelChunkHeightmaps, LevelChunkWithLight, S2cPlayPacket, SectionBlocksUpdate,
};
use crate::protocol::{Encode, Nbt, VarInt, VarLong};
use crate::{BiomeId, BlockPos, ChunkPos, DimensionId, Server, Ticks};

pub struct Chunks {
    chunks: HashMap<ChunkPos, Chunk>,
    server: Server,
    dimension: DimensionId,
}

impl Chunks {
    pub(crate) fn new(server: Server, dimension: DimensionId) -> Self {
        Self {
            chunks: HashMap::new(),
            server,
            dimension,
        }
    }

    pub fn create(&mut self, pos: impl Into<ChunkPos>) -> bool {
        let section_count = (self.server.dimension(self.dimension).height / 16) as u32;
        let chunk = Chunk::new(section_count, self.server.current_tick());
        self.chunks.insert(pos.into(), chunk).is_none()
    }

    pub fn delete(&mut self, pos: ChunkPos) -> bool {
        self.chunks.remove(&pos).is_some()
    }

    pub fn count(&self) -> usize {
        self.chunks.len()
    }

    pub fn get(&self, pos: impl Into<ChunkPos>) -> Option<&Chunk> {
        self.chunks.get(&pos.into())
    }

    pub fn get_mut(&mut self, pos: impl Into<ChunkPos>) -> Option<&mut Chunk> {
        self.chunks.get_mut(&pos.into())
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
    }

    pub fn iter(&self) -> impl FusedIterator<Item = (ChunkPos, &Chunk)> + Clone + '_ {
        self.chunks.iter().map(|(&pos, chunk)| (pos, chunk))
    }

    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (ChunkPos, &mut Chunk)> + '_ {
        self.chunks.iter_mut().map(|(&pos, chunk)| (pos, chunk))
    }

    pub fn par_iter(&self) -> impl ParallelIterator<Item = (ChunkPos, &Chunk)> + Clone + '_ {
        self.chunks.par_iter().map(|(&pos, chunk)| (pos, chunk))
    }

    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (ChunkPos, &mut Chunk)> + '_ {
        self.chunks.par_iter_mut().map(|(&pos, chunk)| (pos, chunk))
    }

    pub fn get_block_state(&self, pos: impl Into<BlockPos>) -> Option<BlockState> {
        let pos = pos.into();
        let chunk_pos = ChunkPos::from(pos);

        let chunk = self.get(chunk_pos)?;

        let min_y = self.server.dimension(self.dimension).min_y;

        let y = pos.y.checked_sub(min_y)?.try_into().ok()?;

        if y < chunk.height() {
            Some(chunk.get_block_state(
                pos.x.rem_euclid(16) as usize,
                y,
                pos.z.rem_euclid(16) as usize,
            ))
        } else {
            None
        }
    }

    pub fn set_block_state(&mut self, pos: impl Into<BlockPos>, block: BlockState) -> bool {
        let pos = pos.into();
        let chunk_pos = ChunkPos::from(pos);

        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            let min_y = self.server.dimension(self.dimension).min_y;

            if let Some(y) = pos.y.checked_sub(min_y).and_then(|y| y.try_into().ok()) {
                if y < chunk.height() {
                    chunk.set_block_state(
                        pos.x.rem_euclid(16) as usize,
                        y,
                        pos.z.rem_euclid(16) as usize,
                        block,
                    );
                    return true;
                }
            }
        }

        false
    }
}

pub struct Chunk {
    sections: Box<[ChunkSection]>,
    // TODO block_entities: HashMap<u32, BlockEntity>,
    /// The MOTION_BLOCKING heightmap
    heightmap: Vec<i64>,
    created_tick: Ticks,
}

impl Chunk {
    pub(crate) fn new(section_count: u32, current_tick: Ticks) -> Self {
        let sect = ChunkSection {
            blocks: [BlockState::AIR.to_raw(); 4096],
            modified_count: 1, // Must be >0 so the chunk is initialized.
            biomes: [BiomeId::default(); 64],
            compact_data: Vec::new(),
        };

        let mut chunk = Self {
            sections: vec![sect; section_count as usize].into(),
            heightmap: Vec::new(),
            created_tick: current_tick,
        };

        chunk.apply_modifications();
        chunk
    }

    pub fn created_tick(&self) -> Ticks {
        self.created_tick
    }

    pub fn height(&self) -> usize {
        self.sections.len() * 16
    }

    pub fn get_block_state(&self, x: usize, y: usize, z: usize) -> BlockState {
        if x < 16 && y < self.height() && z < 16 {
            BlockState::from_raw_unchecked(
                self.sections[y / 16].blocks[x + z * 16 + y % 16 * 16 * 16] & BLOCK_STATE_MASK,
            )
        } else {
            BlockState::AIR
        }
    }

    pub fn set_block_state(&mut self, x: usize, y: usize, z: usize, block: BlockState) {
        assert!(
            x < 16 && y < self.height() && z < 16,
            "the chunk block coordinates must be within bounds"
        );

        let sect = &mut self.sections[y / 16];
        let idx = x + z * 16 + y % 16 * 16 * 16;

        if block.to_raw() != sect.blocks[idx] & BLOCK_STATE_MASK {
            if sect.blocks[idx] & !BLOCK_STATE_MASK == 0 {
                sect.modified_count += 1;
            }
            sect.blocks[idx] = block.to_raw() | !BLOCK_STATE_MASK;

            // TODO: if the block type was modified and the old block type
            // could be a block entity, then the block entity at this
            // position must be cleared.
        }
    }

    pub fn get_biome(&self, x: usize, y: usize, z: usize) -> BiomeId {
        if x < 4 && y < self.height() / 4 && z < 4 {
            self.sections[y / 4].biomes[x + z * 4 + y % 4 * 4 * 4]
        } else {
            BiomeId::default()
        }
    }

    pub fn set_biome(&mut self, x: usize, y: usize, z: usize, b: BiomeId) {
        assert!(
            x < 4 && y < self.height() / 4 && z < 4,
            "the chunk biome coordinates must be within bounds"
        );

        self.sections[y / 4].biomes[x + z * 4 + y % 4 * 4 * 4] = b;
    }

    /// Gets the chunk data packet for this chunk with the given position. This
    /// does not include unapplied changes.
    pub(crate) fn chunk_data_packet(&self, pos: ChunkPos) -> LevelChunkWithLight {
        let mut blocks_and_biomes = Vec::new();

        for sect in self.sections.iter() {
            blocks_and_biomes.extend_from_slice(&sect.compact_data);
        }

        LevelChunkWithLight {
            chunk_x: pos.x,
            chunk_z: pos.z,
            heightmaps: Nbt(LevelChunkHeightmaps {
                motion_blocking: self.heightmap.clone(),
            }),
            blocks_and_biomes,
            block_entities: Vec::new(), // TODO
            trust_edges: true,
            // sky_light_mask: bitvec![u64, _; 1; section_count + 2],
            sky_light_mask: BitVec::new(),
            block_light_mask: BitVec::new(),
            empty_sky_light_mask: BitVec::new(),
            empty_block_light_mask: BitVec::new(),
            // sky_light_arrays: vec![[0xff; 2048]; section_count + 2],
            sky_light_arrays: Vec::new(),
            block_light_arrays: Vec::new(),
        }
    }

    /// Returns unapplied changes to this chunk as block change packets through
    /// the provided closure.
    pub(crate) fn block_change_packets(
        &self,
        pos: ChunkPos,
        min_y: i32,
        mut packet: impl FnMut(BlockChangePacket),
    ) {
        for (sect_y, sect) in self.sections.iter().enumerate() {
            if sect.modified_count == 1 {
                let (idx, &block) = sect
                    .blocks
                    .iter()
                    .enumerate()
                    .find(|&(_, &b)| b & !BLOCK_STATE_MASK != 0)
                    .expect("invalid modified count");

                let global_x = pos.x * 16 + (idx % 16) as i32;
                let global_y = sect_y as i32 * 16 + (idx / (16 * 16)) as i32 + min_y;
                let global_z = pos.z * 16 + (idx / 16 % 16) as i32;

                packet(BlockChangePacket::Single(BlockUpdate {
                    location: BlockPos::new(global_x, global_y, global_z),
                    block_id: VarInt((block & BLOCK_STATE_MASK).into()),
                }));
            } else if sect.modified_count > 1 {
                let mut blocks = Vec::new();
                for y in 0..16 {
                    for z in 0..16 {
                        for x in 0..16 {
                            let block =
                                sect.blocks[x as usize + z as usize * 16 + y as usize * 16 * 16];

                            if block & !BLOCK_STATE_MASK != 0 {
                                blocks.push(VarLong(
                                    ((block & BLOCK_STATE_MASK) as i64) << 12
                                        | (x << 8 | z << 4 | y),
                                ))
                            }
                        }
                    }
                }

                let chunk_section_position = (pos.x as i64) << 42
                    | (pos.z as i64 & 0x3fffff) << 20
                    | (sect_y as i64 + min_y.div_euclid(16) as i64) & 0xfffff;

                packet(BlockChangePacket::Multi(SectionBlocksUpdate {
                    chunk_section_position,
                    invert_trust_edges: false,
                    blocks,
                }));
            }
        }
    }

    pub(crate) fn apply_modifications(&mut self) {
        let mut any_modified = false;

        for sect in self.sections.iter_mut() {
            if sect.modified_count > 0 {
                sect.modified_count = 0;
                any_modified = true;

                sect.compact_data.clear();

                let mut non_air_block_count: i16 = 0;

                for b in &mut sect.blocks {
                    *b &= BLOCK_STATE_MASK;
                    if !BlockState::from_raw_unchecked(*b).is_air() {
                        non_air_block_count += 1;
                    }
                }

                non_air_block_count.encode(&mut sect.compact_data).unwrap();

                encode_paletted_container(
                    sect.blocks.iter().cloned(),
                    4,
                    9,
                    log2_ceil((BlockState::max_raw() + 1) as usize),
                    &mut sect.compact_data,
                )
                .unwrap();

                // TODO: The direct bits per idx changes depending on the number of biomes in
                // the biome registry.
                encode_paletted_container(
                    sect.biomes.iter().map(|b| b.0),
                    0,
                    4,
                    6,
                    &mut sect.compact_data,
                )
                .unwrap();
            }
        }

        if any_modified {
            build_heightmap(&self.sections, &mut self.heightmap);
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum BlockChangePacket {
    Single(BlockUpdate),
    Multi(SectionBlocksUpdate),
}

impl From<BlockChangePacket> for S2cPlayPacket {
    fn from(p: BlockChangePacket) -> Self {
        match p {
            BlockChangePacket::Single(p) => p.into(),
            BlockChangePacket::Multi(p) => p.into(),
        }
    }
}

/// A 16x16x16 section of blocks, biomes, and light in a chunk.
#[derive(Clone)]
struct ChunkSection {
    /// The block states in this section, stored in x, z, y order.
    /// The most significant bit is used to indicate if this block has been
    /// modified.
    blocks: [u16; 4096],
    /// The number of modified blocks
    modified_count: u16,
    biomes: [BiomeId; 64],
    compact_data: Vec<u8>,
}

const BLOCK_STATE_MASK: u16 = 0x7fff;

const _: () = assert!(
    BlockState::max_raw() <= BLOCK_STATE_MASK,
    "There is not enough space in the block state type to store the modified bit. A bit array \
     separate from the block state array should be created to keep track of modified blocks in \
     the chunk section."
);

/// Builds the MOTION_BLOCKING heightmap.
fn build_heightmap(sections: &[ChunkSection], heightmap: &mut Vec<i64>) {
    let height = sections.len() * 16;
    let bits_per_val = log2_ceil(height);
    let vals_per_u64 = 64 / bits_per_val;
    let u64_count = Integer::div_ceil(&256, &vals_per_u64);

    heightmap.clear();
    heightmap.resize(u64_count, 0);

    for x in 0..16 {
        for z in 0..16 {
            for y in (0..height).rev() {
                let block = BlockState::from_raw_unchecked(
                    sections[y / 16].blocks[x + z * 16 + y % 16 * 16 * 16] & BLOCK_STATE_MASK,
                );

                // TODO: is_solid || is_fluid heuristic for motion blocking.
                if !block.is_air() {
                    let column_height = y as u64;

                    let i = x * 16 + z; // TODO: X or Z major?
                    heightmap[i / vals_per_u64] |=
                        (column_height << (i % vals_per_u64 * bits_per_val)) as i64;

                    break;
                }
            }
        }
    }
}

fn encode_paletted_container(
    mut entries: impl ExactSizeIterator<Item = u16> + Clone,
    min_bits_per_idx: usize,
    direct_threshold: usize,
    direct_bits_per_idx: usize,
    w: &mut impl Write,
) -> anyhow::Result<()> {
    let mut palette = Vec::new();

    for entry in entries.clone() {
        if !palette.contains(&entry) {
            palette.push(entry);
        }
    }

    let bits_per_idx = log2_ceil(palette.len());

    (bits_per_idx as u8).encode(w)?;

    if bits_per_idx == 0 {
        // Single value case
        debug_assert_eq!(palette.len(), 1);
        VarInt(palette[0] as i32).encode(w)?;
        VarInt(0).encode(w)?; // data array length
    } else if bits_per_idx >= direct_threshold {
        // Direct case
        // Skip the palette
        let idxs_per_u64 = 64 / direct_bits_per_idx;
        let u64_count = Integer::div_ceil(&entries.len(), &idxs_per_u64);

        VarInt(u64_count as i32).encode(w)?;

        for _ in 0..idxs_per_u64 {
            let mut val = 0u64;
            for i in 0..idxs_per_u64 {
                if let Some(entry) = entries.next() {
                    val |= (entry as u64) << (i * direct_bits_per_idx);
                }
            }
            val.encode(w)?;
        }
    } else {
        // Indirect case
        VarInt(palette.len() as i32).encode(w)?;
        for &val in &palette {
            VarInt(val as i32).encode(w)?;
        }

        let bits_per_idx = bits_per_idx.max(min_bits_per_idx);
        let idxs_per_u64 = 64 / bits_per_idx;
        let u64_count = Integer::div_ceil(&entries.len(), &idxs_per_u64);

        VarInt(u64_count as i32).encode(w)?;

        for _ in 0..u64_count {
            let mut val = 0u64;
            for i in 0..idxs_per_u64 {
                if let Some(entry) = entries.next() {
                    let palette_idx = palette
                        .iter()
                        .position(|&e| e == entry)
                        .expect("entry should be in the palette")
                        as u64;

                    val |= palette_idx << (i * bits_per_idx);
                }
            }
            val.encode(w)?;
        }
    }

    Ok(())
}

/// Calculates the log base 2 rounded up.
fn log2_ceil(n: usize) -> usize {
    n.next_power_of_two().trailing_zeros() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get() {
        let mut chunk = Chunk::new(16, 0);

        chunk.set_block_state(1, 2, 3, BlockState::CAKE);
        assert_eq!(chunk.get_block_state(1, 2, 3), BlockState::CAKE);

        chunk.set_biome(1, 2, 3, BiomeId(7));
        assert_eq!(chunk.get_biome(1, 2, 3), BiomeId(7));
    }

    #[test]
    #[should_panic]
    fn block_state_oob() {
        let mut chunk = Chunk::new(16, 0);

        chunk.set_block_state(16, 0, 0, BlockState::CAKE);
    }

    #[test]
    #[should_panic]
    fn biome_oob() {
        let mut chunk = Chunk::new(16, 0);

        chunk.set_biome(4, 0, 0, BiomeId(0));
    }
}
