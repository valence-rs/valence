use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::path::{Path, PathBuf};

use byteorder::{BigEndian, ByteOrder};
use region::Region;
use tokio::fs::File;
use tokio::sync::{Mutex, MutexGuard};
use valence::biome::BiomeId;
use valence::chunk::{ChunkPos, UnloadedChunk};
use valence::config::Config;
use valence::ident::Ident;
use valence::server::SharedServer;

use crate::error::Error;

pub mod error;
pub mod biome;
pub mod compression;

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
    ///         let world_folder = PathBuf::from_str(WORLD_FOLDER).unwrap();
    ///         server.worlds.insert(
    ///             DimensionId::default(),
    ///             AnvilWorld::new(world_folder, &server.shared),
    ///         );
    ///     }
    /// }
    /// ```
    pub fn new<C: Config>(directory: PathBuf, server: &SharedServer<C>) -> Self {
        let mut biomes = BTreeMap::new();
        for (id, biome) in server.biomes() {
            biomes.insert(biome.name.clone(), id);
        }
        Self {
            world_root: directory,
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
    ///         blank_chunk.set_block_state(
    ///             0,
    ///             0,
    ///             0,
    ///             valence::block::BlockState::from_kind(valence::block::BlockKind::Lava),
    ///         );
    ///         world.chunks.insert(pos, blank_chunk, true);
    ///     }
    /// }
    /// ```
    pub async fn load_chunks<I: Iterator<Item = ChunkPos>>(
        &self,
        positions: I,
    ) -> Result<impl IntoIterator<Item = (ChunkPos, Option<UnloadedChunk>)>, Error> {
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

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Eq, Ord)]
pub struct RegionPos {
    x: i32,
    z: i32,
}

impl From<ChunkPos> for RegionPos {
    fn from(pos: ChunkPos) -> Self {
        Self {
            x: pos.x >> 5,
            z: pos.z >> 5,
        }
    }
}

impl RegionPos {
    pub fn path(self, world_root: impl AsRef<Path>) -> PathBuf {
        world_root
            .as_ref()
            .join("region")
            .join(format!("r.{}.{}.mca", self.x, self.z))
    }

    pub fn contains(self, chunk_pos: ChunkPos) -> bool {
        Self::from(chunk_pos) == self
    }
}

/// The location of the chunk inside the region file.
#[derive(Copy, Clone, Debug)]
struct ChunkSeekLocation {
    offset_sectors: u32,
    len_sectors: u8,
}

impl ChunkSeekLocation {
    const fn zero() -> Self {
        Self {
            offset_sectors: 0,
            len_sectors: 0,
        }
    }

    const fn offset(&self) -> u64 {
        self.offset_sectors as u64 * 1024 * 4
    }

    const fn len(&self) -> usize {
        self.len_sectors as usize * 1024 * 4
    }

    fn load(&mut self, chunk: [u8; 4]) {
        self.offset_sectors = BigEndian::read_u24(&chunk[..3]);
        self.len_sectors = chunk[3];
    }
}

/// The timestamp when the chunk was last modified in seconds since epoch.
#[derive(Copy, Clone)]
pub struct ChunkTimestamp(u32);

impl Debug for ChunkTimestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}s", self.0)
    }
}

impl ChunkTimestamp {
    const fn zero() -> Self {
        Self(0)
    }

    fn load(&mut self, chunk: [u8; 4]) {
        self.0 = BigEndian::read_u32(&chunk)
    }

    fn into_option(self) -> Option<Self> {
        if self.0 == 0 {
            None
        } else {
            Some(self)
        }
    }

    #[inline(always)]
    pub fn seconds_since_epoch(self) -> u32 {
        self.0
    }
}
