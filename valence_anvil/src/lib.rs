use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::PathBuf;

use region::{ChunkTimestamp, Region, RegionPos};
use tokio::fs::File;
use tokio::sync::{Mutex, MutexGuard};
use valence::biome::{Biome, BiomeId};
use valence::chunk::{ChunkPos, UnloadedChunk};
use valence::config::Config;
use valence::ident::Ident;

use crate::error::Error;

pub mod biome;
pub mod compression;
pub mod error;

mod palette;
mod region;

#[derive(Debug)]
pub struct AnvilWorld {
    world_root: PathBuf,
    biomes: BTreeMap<Ident<String>, BiomeId>,
    region_files: Mutex<BTreeMap<RegionPos, Option<Region<File>>>>,
}

impl AnvilWorld {
    //noinspection ALL
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
    /// ```
    /// impl Config for Game {
    ///     fn init(&self, server: &mut Server<Self>) {
    ///         server.worlds.insert(
    ///             DimensionId::default(),
    ///             AnvilWorld::new::<Game>(&self.world_dir, server.shared.biomes()),
    ///         );
    ///     }
    /// }
    /// ```
    pub fn new<'a, C: Config>(
        directory: impl Into<PathBuf>,
        server_biomes: impl Iterator<Item = (BiomeId, &'a Biome)>,
    ) -> Self {
        let mut biomes = BTreeMap::new();
        for (id, biome) in server_biomes {
            biomes.insert(biome.name.clone(), id);
        }
        Self {
            world_root: directory.into(),
            biomes,
            region_files: Mutex::new(BTreeMap::new()),
        }
    }

    //noinspection ALL
    /// Load chunks from the available region files within the world directory.
    /// This operation will temporarily block operations on all region files
    /// within `AnvilWorld`.
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
    /// ```
    /// use valence::prelude::*;
    ///
    /// let to_load = chunks_in_view_distance(ChunkPos::at(p.x, p.z), dist);
    /// let future = world.state.load_chunks(to_load);
    /// let parsed_chunks = futures::executor::block_on(future).unwrap();
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
    pub async fn load_chunks<I: Iterator<Item = ChunkPos>>(
        &self,
        positions: I,
    ) -> Result<impl Iterator<Item = (ChunkPos, Option<UnloadedChunk>)>, Error> {
        let mut map = BTreeMap::<RegionPos, Vec<ChunkPos>>::new();
        for pos in positions {
            let region_pos = RegionPos::from(pos);
            map.entry(region_pos)
                .and_modify(|v| v.push(pos))
                .or_insert_with(|| vec![pos]);
        }

        let mut result_vec = Vec::<(ChunkPos, Option<UnloadedChunk>)>::new();
        let mut lock = self.region_files.lock().await;
        for (region_pos, chunk_pos_vec) in map.into_iter() {
            if let Some(region) = self.access_region_mut(&mut lock, region_pos).await? {
                // A region file exists, and it is loaded.
                result_vec.extend(region.parse_chunks(self, chunk_pos_vec).await?);
            } else {
                // No region file exists, there is no data to load here.
                result_vec.extend(chunk_pos_vec.into_iter().map(|pos| (pos, None)));
            }
        }

        Ok(result_vec.into_iter())
    }

    /// Get the last time the chunk was modified in seconds since epoch.
    /// This operation will temporarily block operations on all region files
    /// within `AnvilWorld`.
    ///
    /// # Arguments
    ///
    /// * `positions`: An iterator of chunk positions
    ///
    /// returns: An iterator with `ChunkPos` and the respective
    /// `Option<ChunkTimestamp>` as tuple.
    pub async fn chunk_timestamps<I: Iterator<Item = ChunkPos>>(
        &self,
        positions: I,
    ) -> Result<impl IntoIterator<Item = (ChunkPos, Option<ChunkTimestamp>)>, Error> {
        let mut map = BTreeMap::<RegionPos, Vec<ChunkPos>>::new();
        for pos in positions {
            let region_pos = RegionPos::from(pos);
            map.entry(region_pos)
                .and_modify(|v| v.push(pos))
                .or_insert_with(|| vec![pos]);
        }

        let mut result_vec = Vec::<(ChunkPos, Option<ChunkTimestamp>)>::new();
        let mut lock = self.region_files.lock().await;
        for (region_pos, chunk_pos_vec) in map.into_iter() {
            if let Some(region) = self.access_region_mut(&mut lock, region_pos).await? {
                for chunk_pos in chunk_pos_vec {
                    result_vec.push((chunk_pos, region.chunk_timestamp(chunk_pos)));
                }
            } else {
                for chunk_pos in chunk_pos_vec {
                    result_vec.push((chunk_pos, None));
                }
            }
        }
        Ok(result_vec.into_iter())
    }

    async fn access_region_mut<'a>(
        &self,
        lock: &'a mut MutexGuard<'_, BTreeMap<RegionPos, Option<Region<File>>>>,
        region_pos: RegionPos,
    ) -> Result<Option<&'a mut Region<File>>, Error> {
        Ok(lock
            .entry(region_pos)
            .or_insert({
                let path = region_pos.path(&self.world_root);
                if path.exists() {
                    Some(Region::from_file(File::open(&path).await?, region_pos).await?)
                } else {
                    None
                }
            })
            .as_mut())
    }
}
