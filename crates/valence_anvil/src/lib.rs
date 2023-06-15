#![doc = include_str!("../README.md")]
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::bare_urls,
    rustdoc::invalid_html_tags
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    unreachable_pub,
    clippy::dbg_macro
)]

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::thread;

use anyhow::{bail, ensure};
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use byteorder::{BigEndian, ReadBytesExt};
use flate2::bufread::{GzDecoder, ZlibDecoder};
use flume::{Receiver, Sender};
use lru::LruCache;
use tracing::warn;
use valence_biome::{BiomeId, BiomeRegistry};
use valence_client::{Client, OldView, UpdateClientsSet, View};
use valence_core::chunk_pos::ChunkPos;
use valence_core::ident::Ident;
use valence_entity::{Location, OldLocation};
use valence_instance::{Chunk, Instance};
use valence_nbt::Compound;

mod parse_chunk;

#[derive(Component, Debug)]
pub struct AnvilLevel {
    /// Chunk worker state to be moved to another thread.
    worker_state: Option<ChunkWorkerState>,
    /// The set of chunk positions that should not be loaded or unloaded by
    /// the anvil system.
    ///
    /// This set is empty by default, but you can modify it at any time.
    pub ignored_chunks: HashSet<ChunkPos>,
    /// Chunks that need to be loaded. Chunks with `None` priority have already
    /// been sent to the anvil thread.
    pending: HashMap<ChunkPos, Option<Priority>>,
    /// Sender for the chunk worker thread.
    sender: Sender<ChunkPos>,
    /// Receiver for the chunk worker thread.
    receiver: Receiver<(ChunkPos, WorkerResult)>,
}

type WorkerResult = anyhow::Result<Option<(Chunk, AnvilChunk)>>;

impl AnvilLevel {
    pub fn new(world_root: impl Into<PathBuf>, biomes: &BiomeRegistry) -> Self {
        let mut region_root = world_root.into();
        region_root.push("region");

        let (pending_sender, pending_receiver) = flume::unbounded();
        let (finished_sender, finished_receiver) = flume::bounded(4096);

        Self {
            worker_state: Some(ChunkWorkerState {
                regions: LruCache::new(LRU_CACHE_SIZE),
                region_root,
                sender: finished_sender,
                receiver: pending_receiver,
                decompress_buf: vec![],
                biome_to_id: biomes
                    .iter()
                    .map(|(id, name, _)| (name.to_string_ident(), id))
                    .collect(),
                section_count: 0, // Assigned later.
            }),
            ignored_chunks: HashSet::new(),
            pending: HashMap::new(),
            sender: pending_sender,
            receiver: finished_receiver,
        }
    }

    /// Forces a chunk to be loaded at a specific position in this world. This
    /// will bypass [`AnvilLevel::ignored_chunks`].
    /// Note that the chunk will be unloaded next tick unless it has been added
    /// to [`AnvilLevel::ignored_chunks`] or it is in view of a client.
    ///
    /// This has no effect if a chunk at the position is already present.
    pub fn force_chunk_load(&mut self, pos: ChunkPos) {
        match self.pending.entry(pos) {
            Entry::Occupied(oe) => {
                // If the chunk is already scheduled to load but hasn't been sent to the chunk
                // worker yet, then give it the highest priority.
                if let Some(priority) = oe.into_mut() {
                    *priority = 0;
                }
            }
            Entry::Vacant(ve) => {
                ve.insert(Some(0));
            }
        }
    }
}

const LRU_CACHE_SIZE: NonZeroUsize = match NonZeroUsize::new(256) {
    Some(n) => n,
    None => unreachable!(),
};

/// The order in which chunks should be processed by the anvil worker. Smaller
/// values are sent first.
type Priority = u64;

#[derive(Debug)]
struct ChunkWorkerState {
    /// Region files. An LRU cache is used to limit the number of open file
    /// handles.
    regions: LruCache<RegionPos, RegionEntry>,
    /// Path to the "region" subdirectory in the world root.
    region_root: PathBuf,
    /// Sender of finished chunks.
    sender: Sender<(ChunkPos, WorkerResult)>,
    /// Receiver of pending chunks.
    receiver: Receiver<ChunkPos>,
    /// Scratch buffer for decompression.
    decompress_buf: Vec<u8>,
    /// Mapping of biome names to their biome ID.
    biome_to_id: BTreeMap<Ident<String>, BiomeId>,
    /// Number of chunk sections in the instance.
    section_count: usize,
}

impl ChunkWorkerState {
    fn get_chunk(&mut self, pos: ChunkPos) -> anyhow::Result<Option<AnvilChunk>> {
        let region_x = pos.x.div_euclid(32);
        let region_z = pos.z.div_euclid(32);

        let region = match self.regions.get_mut(&(region_x, region_z)) {
            Some(RegionEntry::Occupied(region)) => region,
            Some(RegionEntry::Vacant) => return Ok(None),
            None => {
                let path = self
                    .region_root
                    .join(format!("r.{region_x}.{region_z}.mca"));

                let mut file = match File::options().read(true).write(true).open(path) {
                    Ok(file) => file,
                    Err(e) if e.kind() == ErrorKind::NotFound => {
                        self.regions.put((region_x, region_z), RegionEntry::Vacant);
                        return Ok(None);
                    }
                    Err(e) => return Err(e.into()),
                };

                let mut header = [0; SECTOR_SIZE * 2];

                file.read_exact(&mut header)?;

                // TODO: this is ugly.
                let res = self.regions.get_or_insert_mut((region_x, region_z), || {
                    RegionEntry::Occupied(Region { file, header })
                });

                match res {
                    RegionEntry::Occupied(r) => r,
                    RegionEntry::Vacant => unreachable!(),
                }
            }
        };

        let chunk_idx = (pos.x.rem_euclid(32) + pos.z.rem_euclid(32) * 32) as usize;

        let location_bytes = (&region.header[chunk_idx * 4..]).read_u32::<BigEndian>()?;
        let timestamp = (&region.header[chunk_idx * 4 + SECTOR_SIZE..]).read_u32::<BigEndian>()?;

        if location_bytes == 0 {
            // No chunk exists at this position.
            return Ok(None);
        }

        let sector_offset = (location_bytes >> 8) as u64;
        let sector_count = (location_bytes & 0xff) as usize;

        // If the sector offset was <2, then the chunk data would be inside the region
        // header. That doesn't make any sense.
        ensure!(sector_offset >= 2, "invalid chunk sector offset");

        // Seek to the beginning of the chunk's data.
        region
            .file
            .seek(SeekFrom::Start(sector_offset * SECTOR_SIZE as u64))?;

        let exact_chunk_size = region.file.read_u32::<BigEndian>()? as usize;

        // size of this chunk in sectors must always be >= the exact size.
        ensure!(
            sector_count * SECTOR_SIZE >= exact_chunk_size,
            "invalid chunk size"
        );

        let mut data_buf = vec![0; exact_chunk_size].into_boxed_slice();
        region.file.read_exact(&mut data_buf)?;

        let mut r = data_buf.as_ref();

        self.decompress_buf.clear();

        // What compression does the chunk use?
        let mut nbt_slice = match r.read_u8()? {
            // GZip
            1 => {
                let mut z = GzDecoder::new(r);
                z.read_to_end(&mut self.decompress_buf)?;
                self.decompress_buf.as_slice()
            }
            // Zlib
            2 => {
                let mut z = ZlibDecoder::new(r);
                z.read_to_end(&mut self.decompress_buf)?;
                self.decompress_buf.as_slice()
            }
            // Uncompressed
            3 => r,
            // Unknown
            b => bail!("unknown compression scheme number of {b}"),
        };

        let (data, _) = Compound::from_binary(&mut nbt_slice)?;

        ensure!(nbt_slice.is_empty(), "not all chunk NBT data was read");

        Ok(Some(AnvilChunk { data, timestamp }))
    }
}

struct AnvilChunk {
    data: Compound,
    timestamp: u32,
}

/// X and Z positions of a region.
type RegionPos = (i32, i32);

#[allow(clippy::large_enum_variant)] // We're not moving this around.
#[derive(Debug)]
enum RegionEntry {
    /// There is a region file loaded here.
    Occupied(Region),
    /// There is no region file at this position. Don't try to read it from the
    /// filesystem again.
    Vacant,
}

#[derive(Debug)]
struct Region {
    file: File,
    /// The first 8 KiB in the file.
    header: [u8; SECTOR_SIZE * 2],
}

const SECTOR_SIZE: usize = 4096;

pub struct AnvilPlugin;

impl Plugin for AnvilPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ChunkLoadEvent>()
            .add_event::<ChunkUnloadEvent>()
            .add_system(remove_unviewed_chunks.in_base_set(CoreSet::PreUpdate))
            .add_systems(
                (init_anvil, update_client_views, send_recv_chunks)
                    .chain()
                    .in_base_set(CoreSet::PostUpdate)
                    .before(UpdateClientsSet),
            );
    }
}

fn init_anvil(mut query: Query<(&mut AnvilLevel, &Instance), Added<AnvilLevel>>) {
    for (mut level, inst) in &mut query {
        if let Some(mut state) = level.worker_state.take() {
            state.section_count = inst.section_count();
            thread::spawn(move || anvil_worker(state));
        }
    }
}

/// Removes all chunks no longer viewed by clients.
///
/// This needs to run in `PreUpdate` where the chunk viewer counts have been
/// updated from the previous tick.
fn remove_unviewed_chunks(
    mut instances: Query<(Entity, &mut Instance, &AnvilLevel)>,
    mut unload_events: EventWriter<ChunkUnloadEvent>,
) {
    for (entity, mut inst, anvil) in &mut instances {
        inst.retain_chunks(|pos, chunk| {
            if chunk.is_viewed_mut() || anvil.ignored_chunks.contains(&pos) {
                true
            } else {
                unload_events.send(ChunkUnloadEvent {
                    instance: entity,
                    pos,
                });
                false
            }
        });
    }
}

fn update_client_views(
    clients: Query<(&Location, Ref<OldLocation>, View, OldView), With<Client>>,
    mut instances: Query<(&Instance, &mut AnvilLevel)>,
) {
    for (loc, old_loc, view, old_view) in &clients {
        let view = view.get();
        let old_view = old_view.get();

        if loc != &*old_loc || view != old_view || old_loc.is_added() {
            let Ok((inst, mut anvil)) = instances.get_mut(loc.0) else {
                continue
            };

            let queue_pos = |pos| {
                if !anvil.ignored_chunks.contains(&pos) && inst.chunk(pos).is_none() {
                    // Chunks closer to clients are prioritized.
                    match anvil.pending.entry(pos) {
                        Entry::Occupied(mut oe) => {
                            if let Some(priority) = oe.get_mut() {
                                let dist = view.pos.distance_squared(pos);
                                *priority = (*priority).min(dist);
                            }
                        }
                        Entry::Vacant(ve) => {
                            let dist = view.pos.distance_squared(pos);
                            ve.insert(Some(dist));
                        }
                    }
                }
            };

            // Queue all the new chunks in the view to be sent to the anvil worker.
            if old_loc.is_added() {
                view.iter().for_each(queue_pos);
            } else {
                view.diff(old_view).for_each(queue_pos);
            }
        }
    }
}

fn send_recv_chunks(
    mut instances: Query<(Entity, &mut Instance, &mut AnvilLevel)>,
    mut to_send: Local<Vec<(Priority, ChunkPos)>>,
    mut load_events: EventWriter<ChunkLoadEvent>,
) {
    for (entity, mut inst, anvil) in &mut instances {
        let anvil = anvil.into_inner();

        // Insert the chunks that are finished loading into the instance and send load
        // events.
        for (pos, res) in anvil.receiver.drain() {
            anvil.pending.remove(&pos);

            let status = match res {
                Ok(Some((chunk, AnvilChunk { data, timestamp }))) => {
                    inst.insert_chunk(pos, chunk);
                    ChunkLoadStatus::Success { data, timestamp }
                }
                Ok(None) => ChunkLoadStatus::Empty,
                Err(e) => ChunkLoadStatus::Failed(e),
            };

            load_events.send(ChunkLoadEvent {
                instance: entity,
                pos,
                status,
            });
        }

        // Collect all the new chunks that need to be loaded this tick.
        for (pos, priority) in &mut anvil.pending {
            if let Some(pri) = priority.take() {
                to_send.push((pri, *pos));
            }
        }

        // Sort chunks by ascending priority.
        to_send.sort_unstable_by_key(|(pri, _)| *pri);

        // Send the sorted chunks to be loaded.
        for (_, pos) in to_send.drain(..) {
            let _ = anvil.sender.try_send(pos);
        }
    }
}

fn anvil_worker(mut state: ChunkWorkerState) {
    while let Ok(pos) = state.receiver.recv() {
        let res = get_chunk(pos, &mut state);

        let _ = state.sender.send((pos, res));
    }

    fn get_chunk(pos: ChunkPos, state: &mut ChunkWorkerState) -> WorkerResult {
        let Some(anvil_chunk) = state.get_chunk(pos)? else {
            return Ok(None);
        };

        let mut chunk = Chunk::new(state.section_count);
        // TODO: account for min_y correctly.
        parse_chunk::parse_chunk(&anvil_chunk.data, &mut chunk, 4, |biome| {
            state
                .biome_to_id
                .get(biome.as_str())
                .copied()
                .unwrap_or_default()
        })?;

        Ok(Some((chunk, anvil_chunk)))
    }
}

/// An event sent by `valence_anvil` after an attempt to load a chunk is made.
#[derive(Debug)]
pub struct ChunkLoadEvent {
    /// The [`Instance`] where the chunk is located.
    pub instance: Entity,
    /// The position of the chunk in the instance.
    pub pos: ChunkPos,
    pub status: ChunkLoadStatus,
}

#[derive(Debug)]
pub enum ChunkLoadStatus {
    /// A new chunk was successfully loaded and inserted into the instance.
    Success {
        /// The raw chunk data of the new chunk.
        data: Compound,
        /// The time this chunk was last modified, measured in seconds since the
        /// epoch.
        timestamp: u32,
    },
    /// The Anvil level does not have a chunk at the position. No chunk was
    /// loaded.
    Empty,
    /// An attempt was made to load the chunk, but something went wrong.
    Failed(anyhow::Error),
}

/// An event sent by `valence_anvil` when a chunk is unloaded from an instance.
#[derive(Debug)]
pub struct ChunkUnloadEvent {
    /// The [`Instance`] where the chunk was unloaded.
    pub instance: Entity,
    /// The position of the chunk that was unloaded.
    pub pos: ChunkPos,
}
