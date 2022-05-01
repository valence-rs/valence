// TODO: https://github.com/rust-lang/rust/issues/88581 for div_ceil

use std::io::Write;
use std::iter::FusedIterator;

use bitvec::bitvec;
use bitvec::vec::BitVec;
use num::Integer;
use rayon::iter::ParallelIterator;

use crate::block::BlockState;
use crate::glm::DVec2;
use crate::packets::play::{
    BlockChange, ChunkDataAndUpdateLight, ChunkDataHeightmaps, ClientPlayPacket, MultiBlockChange,
};
use crate::protocol::{Encode, Nbt};
use crate::slotmap::{Key, SlotMap};
use crate::var_int::VarInt;
use crate::BiomeId;

pub struct ChunkStore {
    sm: SlotMap<Chunk>,
}

impl ChunkStore {
    pub(crate) fn new() -> Self {
        Self { sm: SlotMap::new() }
    }

    pub fn count(&self) -> usize {
        self.sm.count()
    }

    pub fn create(&mut self, section_count: usize) -> ChunkId {
        ChunkId(self.sm.insert(Chunk::new(section_count)))
    }

    pub fn delete(&mut self, chunk: ChunkId) -> bool {
        self.sm.remove(chunk.0).is_some()
    }

    pub fn get(&self, chunk: ChunkId) -> Option<&Chunk> {
        self.sm.get(chunk.0)
    }

    pub fn get_mut(&mut self, chunk: ChunkId) -> Option<&mut Chunk> {
        self.sm.get_mut(chunk.0)
    }

    pub fn clear(&mut self) {
        self.sm.clear();
    }

    pub fn iter(&self) -> impl FusedIterator<Item = (ChunkId, &Chunk)> + Clone + '_ {
        self.sm.iter().map(|(k, v)| (ChunkId(k), v))
    }

    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (ChunkId, &mut Chunk)> + '_ {
        self.sm.iter_mut().map(|(k, v)| (ChunkId(k), v))
    }

    pub fn par_iter(&self) -> impl ParallelIterator<Item = (ChunkId, &Chunk)> + Clone + '_ {
        self.sm.par_iter().map(|(k, v)| (ChunkId(k), v))
    }

    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (ChunkId, &mut Chunk)> + '_ {
        self.sm.par_iter_mut().map(|(k, v)| (ChunkId(k), v))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ChunkId(Key);

pub struct Chunk {
    sections: Box<[ChunkSection]>,
    // TODO block_entities: HashMap<u32, BlockEntity>,
    heightmap: Vec<i64>,
    modified: bool,
    created_this_tick: bool,
}

impl Chunk {
    pub(crate) fn new(section_count: usize) -> Self {
        let sect = ChunkSection {
            blocks: [BlockState::default(); 4096],
            biomes: [BiomeId::default(); 64],
            compact_data: Vec::new(),
            modified: true,
        };

        let mut chunk = Self {
            sections: vec![sect; section_count].into(),
            heightmap: Vec::new(),
            modified: true,
            created_this_tick: true,
        };

        chunk.apply_modifications();
        chunk
    }

    pub fn created_this_tick(&self) -> bool {
        self.created_this_tick
    }

    pub(crate) fn clear_created_this_tick(&mut self) {
        self.created_this_tick = false;
    }

    pub fn height(&self) -> usize {
        self.sections.len() * 16
    }

    pub fn get_block_state(&self, x: usize, y: usize, z: usize) -> BlockState {
        if x < 16 && y < self.height() && z < 16 {
            self.sections[y / 16].blocks[x + z * 16 + y % 16 * 16 * 16]
        } else {
            BlockState::AIR
        }
    }

    pub fn set_block_state(&mut self, x: usize, y: usize, z: usize, block: BlockState) {
        if x < 16 && y < self.height() && z < 16 {
            let sec = &mut self.sections[y / 16];
            let idx = x + z * 16 + y % 16 * 16 * 16;
            if block != sec.blocks[idx] {
                sec.blocks[idx] = block;
                // TODO: set the modified bit.
                sec.modified = true;
                self.modified = true;
                // TODO: update block entity if b could have block entity data.
            }
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
        if x < 4 && y < self.height() / 4 && z < 4 {
            self.sections[y / 4].biomes[x + z * 4 + y % 4 * 4 * 4] = b;
        }
    }

    /// Gets the chunk data packet with the given number of sections for this
    /// chunk. This does not include unapplied changes.
    pub(crate) fn chunk_data_packet(
        &self,
        pos: ChunkPos,
        section_count: usize,
    ) -> ChunkDataAndUpdateLight {
        let mut blocks_and_biomes = Vec::new();

        for i in 0..section_count {
            match self.sections.get(i) {
                Some(sect) => {
                    blocks_and_biomes.extend_from_slice(&sect.compact_data);
                }
                None => {
                    // Extra chunk sections are encoded as empty with the default biome.

                    // non air block count
                    0i16.encode(&mut blocks_and_biomes).unwrap();
                    // blocks
                    encode_paletted_container_single(0, &mut blocks_and_biomes).unwrap();
                    // biomes
                    encode_paletted_container_single(0, &mut blocks_and_biomes).unwrap();
                }
            }
        }

        let motion_blocking = if section_count == self.sections.len() {
            self.heightmap.clone()
        } else {
            // This is bad for two reasons:
            // - Rebuilding the heightmap from scratch is slow.
            // - The new heightmap is subtly wrong because we are building it from blocks
            //   with the modifications applied already.
            // But you shouldn't be using chunks with heights different than the dimensions
            // they're residing in anyway, so whatever.
            let mut heightmap = Vec::new();
            build_heightmap(&self.sections, &mut heightmap);
            heightmap
        };

        ChunkDataAndUpdateLight {
            chunk_x: pos.x,
            chunk_z: pos.z,
            heightmaps: Nbt(ChunkDataHeightmaps { motion_blocking }),
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

    /// Gets the unapplied changes to this chunk as a block change packet.
    pub(crate) fn block_change_packet(&self, pos: ChunkPos) -> Option<BlockChangePacket> {
        if !self.modified {
            return None;
        }

        // TODO
        None
    }

    pub(crate) fn apply_modifications(&mut self) {
        if self.modified {
            self.modified = false;

            for sect in self.sections.iter_mut() {
                if sect.modified {
                    sect.modified = false;

                    sect.compact_data.clear();

                    let non_air_block_count = sect.blocks.iter().filter(|&&b| !b.is_air()).count();

                    (non_air_block_count as i16)
                        .encode(&mut sect.compact_data)
                        .unwrap();

                    encode_paletted_container(
                        sect.blocks.iter().map(|b| b.to_raw()),
                        4,
                        9,
                        15,
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

            build_heightmap(&self.sections, &mut self.heightmap);
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum BlockChangePacket {
    Single(BlockChange),
    Multi(MultiBlockChange),
}

impl From<BlockChangePacket> for ClientPlayPacket {
    fn from(p: BlockChangePacket) -> Self {
        match p {
            BlockChangePacket::Single(p) => p.into(),
            BlockChangePacket::Multi(p) => p.into(),
        }
    }
}

/// The X and Z position of a chunk in a world.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ChunkPos {
    /// The X position of the chunk.
    pub x: i32,
    /// The Z position of the chunk.
    pub z: i32,
}

impl ChunkPos {
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// Returns the chunk position of the chunk that the given coordinates are
    /// contained within.
    pub fn from_xz(xz: DVec2) -> Self {
        Self {
            x: (xz.x / 16.0) as i32,
            z: (xz.y / 16.0) as i32,
        }
    }
}

impl From<(i32, i32)> for ChunkPos {
    fn from((x, z): (i32, i32)) -> Self {
        ChunkPos { x, z }
    }
}

/// A 16x16x16 section of blocks, biomes, and light in a chunk.
#[derive(Clone)]
struct ChunkSection {
    /// The blocks in this section, stored in x, z, y order.
    blocks: [BlockState; 4096],
    biomes: [BiomeId; 64],
    compact_data: Vec<u8>,
    /// If the blocks or biomes were modified.
    modified: bool,
}

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
                let block = sections[y / 16].blocks[x + z * 16 + y % 16 * 16 * 16];
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
    entries: impl ExactSizeIterator<Item = u16> + Clone,
    min_bits_per_idx: usize,
    direct_threshold: usize,
    direct_bits_per_idx: usize,
    w: &mut impl Write,
) -> anyhow::Result<()> {
    let entries_len = entries.len();

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

        for entry in entries {
            let mut val = 0u64;
            for i in 0..idxs_per_u64 {
                val |= (entry as u64) << (i * direct_bits_per_idx);
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

        for entry in entries {
            let palette_idx = palette
                .iter()
                .position(|&e| e == entry)
                .expect("entry should be in the palette") as u64;

            let mut val = 0u64;
            for i in 0..idxs_per_u64 {
                val |= palette_idx << (i * bits_per_idx);
            }
            val.encode(w)?;
        }
    }

    Ok(())
}

/// Encode a paletted container where all values are the same.
fn encode_paletted_container_single(entry: u16, w: &mut impl Write) -> anyhow::Result<()> {
    0u8.encode(w)?; // bits per idx
    VarInt(entry as i32).encode(w)?; // single value
    VarInt(0).encode(w)?; // data array length
    Ok(())
}

/// Calculates the log base 2 rounded up.
fn log2_ceil(n: usize) -> usize {
    n.next_power_of_two().trailing_zeros() as usize
}
