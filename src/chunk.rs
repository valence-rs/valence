//! Chunks and related types.

// TODO: https://github.com/rust-lang/rust/issues/88581 for div_ceil

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io::Write;
use std::iter::FusedIterator;

use bitvec::vec::BitVec;
use num::Integer;
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::biome::BiomeId;
use crate::block::BlockState;
use crate::block_pos::BlockPos;
pub use crate::chunk_pos::ChunkPos;
use crate::config::Config;
use crate::dimension::DimensionId;
use crate::protocol_inner::packets::s2c::play::{
    BlockUpdate, ChunkData, ChunkDataHeightmaps, ChunkSectionUpdate, S2cPlayPacket,
};
use crate::protocol_inner::{Encode, Nbt, VarInt, VarLong};
use crate::server::SharedServer;
use crate::Ticks;

/// A container for all [`Chunks`]s in a [`World`](crate::world::World).
pub struct Chunks<C: Config> {
    chunks: HashMap<ChunkPos, Chunk<C>>,
    server: SharedServer<C>,
    dimension: DimensionId,
}

impl<C: Config> Chunks<C> {
    pub(crate) fn new(server: SharedServer<C>, dimension: DimensionId) -> Self {
        Self {
            chunks: HashMap::new(),
            server,
            dimension,
        }
    }

    /// Creates an empty chunk at the provided position and returns a mutable
    /// refernce to it.
    ///
    /// If a chunk at the position already exists, then the old chunk
    /// is overwritten.
    ///
    /// **Note**: For the vanilla Minecraft client to see a chunk, all chunks
    /// adjacent to it must also be loaded. It is also important that clients
    /// are not spawned within unloaded chunks via
    /// [`spawn`](crate::client::Client::spawn).
    pub fn create(&mut self, pos: impl Into<ChunkPos>, data: C::ChunkState) -> &mut Chunk<C> {
        let section_count = (self.server.dimension(self.dimension).height / 16) as u32;
        let chunk = Chunk::new(section_count, self.server.current_tick(), data);

        match self.chunks.entry(pos.into()) {
            Entry::Occupied(mut oe) => {
                oe.insert(chunk);
                oe.into_mut()
            }
            Entry::Vacant(ve) => ve.insert(chunk),
        }
    }

    /// Removes a chunk at the provided position.
    ///
    /// If a chunk exists at the position, then it is deleted and `true` is
    /// returned. Otherwise, `false` is returned.
    pub fn delete(&mut self, pos: impl Into<ChunkPos>) -> bool {
        self.chunks.remove(&pos.into()).is_some()
    }

    /// Returns the number of loaded chunks.
    pub fn count(&self) -> usize {
        self.chunks.len()
    }

    /// Gets a shared reference to the chunk at the provided position.
    ///
    /// If there is no chunk at the position, then `None` is returned.
    pub fn get(&self, pos: impl Into<ChunkPos>) -> Option<&Chunk<C>> {
        self.chunks.get(&pos.into())
    }

    /// Gets an exclusive reference to the chunk at the provided position.
    ///
    /// If there is no chunk at the position, then `None` is returned.
    pub fn get_mut(&mut self, pos: impl Into<ChunkPos>) -> Option<&mut Chunk<C>> {
        self.chunks.get_mut(&pos.into())
    }

    /// Deletes all chunks.
    pub fn clear(&mut self) {
        self.chunks.clear();
    }

    /// Returns an immutable iterator over all chunks in the world in an
    /// unspecified order.
    pub fn iter(&self) -> impl FusedIterator<Item = (ChunkPos, &Chunk<C>)> + Clone + '_ {
        self.chunks.iter().map(|(&pos, chunk)| (pos, chunk))
    }

    /// Returns a mutable iterator over all chunks in the world in an
    /// unspecified order.
    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (ChunkPos, &mut Chunk<C>)> + '_ {
        self.chunks.iter_mut().map(|(&pos, chunk)| (pos, chunk))
    }

    /// Returns a parallel immutable iterator over all chunks in the world in an
    /// unspecified order.
    pub fn par_iter(&self) -> impl ParallelIterator<Item = (ChunkPos, &Chunk<C>)> + Clone + '_ {
        self.chunks.par_iter().map(|(&pos, chunk)| (pos, chunk))
    }

    /// Returns a parallel mutable iterator over all chunks in the world in an
    /// unspecified order.
    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (ChunkPos, &mut Chunk<C>)> + '_ {
        self.chunks.par_iter_mut().map(|(&pos, chunk)| (pos, chunk))
    }

    /// Gets the block state at a position.
    ///
    /// If the position is not inside of a chunk, then `None` is returned.
    ///
    /// Note: if you need to get a large number of blocks, it may be more
    /// efficient to read from the chunks directly with
    /// [`Chunk::get_block_state`].
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

    /// Sets the block state at a position.
    ///
    /// If the position is inside of a chunk, then `true` is returned.
    /// Otherwise, `false` is returned.
    ///
    /// Note: if you need to set a large number of blocks, it may be more
    /// efficient write to the chunks directly with
    /// [`Chunk::set_block_state`].
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

/// A chunk is a 16x16-block segment of a world with a height determined by the
/// [`Dimension`](crate::dimension::Dimension) of the world.
///
/// In addition to blocks, chunks also contain [biomes](crate::biome::Biome).
/// Every 4x4x4 segment of blocks in a chunk corresponds to a biome.
pub struct Chunk<C: Config> {
    /// Custom state.
    pub state: C::ChunkState,
    sections: Box<[ChunkSection]>,
    // TODO block_entities: HashMap<u32, BlockEntity>,
    /// The MOTION_BLOCKING heightmap
    heightmap: Vec<i64>,
    created_tick: Ticks,
}

impl<C: Config> Chunk<C> {
    pub(crate) fn new(section_count: u32, current_tick: Ticks, data: C::ChunkState) -> Self {
        let sect = ChunkSection {
            blocks: [BlockState::AIR.to_raw(); 4096],
            modified_count: 1, // Must be >0 so the chunk is initialized.
            biomes: [BiomeId::default(); 64],
            compact_data: Vec::new(),
        };

        let mut chunk = Self {
            state: data,
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
    pub(crate) fn chunk_data_packet(&self, pos: ChunkPos) -> ChunkData {
        let mut blocks_and_biomes = Vec::new();

        for sect in self.sections.iter() {
            blocks_and_biomes.extend_from_slice(&sect.compact_data);
        }

        ChunkData {
            chunk_x: pos.x,
            chunk_z: pos.z,
            heightmaps: Nbt(ChunkDataHeightmaps {
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

                packet(BlockChangePacket::Multi(ChunkSectionUpdate {
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
    Multi(ChunkSectionUpdate),
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
