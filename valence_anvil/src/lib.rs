use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::fs::File;
use std::path::PathBuf;

use region::{ChunkTimestamp, Region, RegionPos};
use valence::biome::{Biome, BiomeId};
use valence::chunk::{ChunkPos, UnloadedChunk};
use valence::config::Config;
use valence::dimension::Dimension;
use valence::protocol::Ident;
use valence::vek::num_traits::FromPrimitive;

use crate::error::Error;

pub mod biome;
pub mod compression;
pub mod error;

mod chunk;
mod palette;
mod region;

#[derive(Debug)]
pub struct AnvilWorld {
    world_root: PathBuf,
    config: AnvilWorldConfig,
    region_files: BTreeMap<RegionPos, Option<Region<File>>>,
}

#[derive(Debug)]
pub struct AnvilWorldConfig {
    pub min_y: isize,
    pub height: usize,
    pub biomes: BTreeMap<Ident<String>, BiomeId>,
}

impl AnvilWorld {
    ///  Creates an `AnvilWorld` instance.
    ///
    /// # Arguments
    ///
    /// * `directory`: A path to the world folder. Inside this folder you should
    ///   see the `region` directory.
    /// * `server`: The shared server. This is used to initialize which biomes
    ///   to use.
    ///
    /// returns: AnvilWorld
    ///
    /// # Examples
    ///
    /// ```ignore
    /// impl Config for Game {
    ///     fn init(&self, server: &mut Server<Self>) {
    ///         for (id, dimension) in server.shared.dimensions() {
    ///             server.worlds.insert(
    ///                 id,
    ///                 AnvilWorld::new::<Game, _>(&dimension, &self.world_dir, server.shared.biomes()),
    ///             );
    ///         }
    ///     }
    /// }
    /// ```
    pub fn new<C: Config, BIOME: Borrow<Biome>>(
        dimension: &Dimension,
        directory: impl Into<PathBuf>,
        server_biomes: impl Iterator<Item = (BiomeId, BIOME)>,
    ) -> Self {
        let mut biomes = BTreeMap::new();
        for (id, biome) in server_biomes {
            biomes.insert(biome.borrow().name.clone(), id);
        }
        Self {
            world_root: directory.into(),
            config: AnvilWorldConfig {
                min_y: isize::from_i32(dimension.min_y)
                    .expect("Dimension min_y could not be converted to isize from i32."),
                height: usize::from_i32(dimension.height)
                    .expect("Dimension height could not be converted to usize from i32."),
                biomes,
            },
            region_files: BTreeMap::new(),
        }
    }

    /// Load chunks from the available region files within the world directory.
    ///
    /// # Arguments
    ///
    /// * `positions`: Any iterator of `valence::chunk_pos::ChunkPos`
    ///
    /// returns: An iterator of the requested chunk positions and their
    /// associated chunks
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use valence::prelude::*;
    ///
    /// let mut new_chunks = Vec::new();
    /// for pos in ChunkPos::at(p.x, p.z).in_view(dist) {
    ///     if let Some(existing) = world.chunks.get_mut(pos) {
    ///         existing.state = true;
    ///     } else {
    ///         new_chunks.push(pos);
    ///     }
    /// }
    ///
    /// let parsed_chunks = world.state.load_chunks(new_chunks.into_iter()).unwrap();
    /// for (pos, chunk) in parsed_chunks {
    ///     if let Some(chunk) = chunk {
    ///         // A chunk has successfully loaded from the region file.
    ///         world.chunks.insert(pos, chunk, true);
    ///     } else {
    ///         // There is no information on this chunk in the region file.
    ///         let mut blank_chunk = UnloadedChunk::new(16);
    ///         blank_chunk.set_block_state(0, 0, 0, BlockState::from_kind(BlockKind::Lava));
    ///         world.chunks.insert(pos, blank_chunk, true);
    ///     }
    /// }
    /// ```
    pub fn load_chunks<I: Iterator<Item = ChunkPos>>(
        &mut self,
        positions: I,
    ) -> Result<impl Iterator<Item = (ChunkPos, Option<UnloadedChunk>)>, Error> {
        let mut region_chunks = BTreeMap::<RegionPos, Vec<ChunkPos>>::new();
        for chunk_pos in positions {
            let region_pos = RegionPos::from(chunk_pos);
            region_chunks
                .entry(region_pos)
                .and_modify(|v| v.push(chunk_pos))
                .or_insert_with(|| vec![chunk_pos]);
        }
        let mut result_vec = Vec::<(ChunkPos, Option<UnloadedChunk>)>::new();
        for (region_pos, chunk_pos_vec) in region_chunks {
            if let Some(region) = self.region_files.entry(region_pos).or_insert({
                let path = region_pos.path(&self.world_root);
                if path.exists() {
                    Some(Region::from_file(File::open(&path)?, region_pos)?)
                } else {
                    None
                }
            }) {
                // A region file exists, and it is loaded.
                result_vec.extend(region.parse_chunks(&self.config, chunk_pos_vec)?);
            } else {
                // No region file exists, there is no data to load here.
                result_vec.extend(chunk_pos_vec.into_iter().map(|pos| (pos, None)));
            }
        }
        Ok(result_vec.into_iter())
    }

    /// Get the last time the chunk was modified in seconds since epoch.
    ///
    /// # Arguments
    ///
    /// * `positions`: An iterator of chunk positions
    ///
    /// returns: An iterator with `ChunkPos` and the respective
    /// `Option<ChunkTimestamp>` as tuple.
    pub fn chunk_timestamps<I: Iterator<Item = ChunkPos>>(
        &mut self,
        positions: I,
    ) -> Result<impl IntoIterator<Item = (ChunkPos, Option<ChunkTimestamp>)>, Error> {
        let mut region_chunks = BTreeMap::<RegionPos, Vec<ChunkPos>>::new();
        for chunk_pos in positions {
            let region_pos = RegionPos::from(chunk_pos);
            region_chunks
                .entry(region_pos)
                .and_modify(|v| v.push(chunk_pos))
                .or_insert_with(|| vec![chunk_pos]);
        }
        let mut result_vec = Vec::<(ChunkPos, Option<ChunkTimestamp>)>::new();
        for (region_pos, chunk_pos_vec) in region_chunks {
            if let Some(region) = self.region_files.entry(region_pos).or_insert({
                let path = region_pos.path(&self.world_root);
                if path.exists() {
                    Some(Region::from_file(File::open(&path)?, region_pos)?)
                } else {
                    None
                }
            }) {
                // A region file exists, and it is loaded.
                for chunk_pos in chunk_pos_vec {
                    result_vec.push((chunk_pos, region.chunk_timestamp(chunk_pos)));
                }
            } else {
                // No region file exists, there is no data to load here.
                for chunk_pos in chunk_pos_vec {
                    result_vec.push((chunk_pos, None));
                }
            }
        }
        Ok(result_vec.into_iter())
    }
}
