use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::thread;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use flume::{Receiver, Sender};
use valence_server::client::{Client, OldView, View};
use valence_server::entity::{EntityLayerId, OldEntityLayerId};
use valence_server::layer::UpdateLayersPreClientSet;
use valence_server::protocol::anyhow;
use valence_server::registry::BiomeRegistry;
use valence_server::{ChunkLayer, ChunkPos};

use crate::parsing::{DimensionFolder, ParsedChunk};

type WorkerResult = anyhow::Result<Option<ParsedChunk>>;

/// The order in which chunks should be processed by the anvil worker. Smaller
/// values are sent first.
type Priority = u64;

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

impl AnvilLevel {
    pub fn new<R: Into<PathBuf>>(world_root: R, biomes: &BiomeRegistry) -> Self {
        let (pending_sender, pending_receiver) = flume::unbounded();
        let (finished_sender, finished_receiver) = flume::bounded(4096);

        Self {
            worker_state: Some(ChunkWorkerState {
                dimension_folder: DimensionFolder::new(world_root, biomes),
                sender: finished_sender,
                receiver: pending_receiver,
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

#[derive(Debug)]
struct ChunkWorkerState {
    /// The world folder containing the region folder where chunks are loaded
    /// from.
    dimension_folder: DimensionFolder,
    /// Sender of finished chunks.
    sender: Sender<(ChunkPos, WorkerResult)>,
    /// Receiver of pending chunks.
    receiver: Receiver<ChunkPos>,
}

pub struct AnvilPlugin;

impl Plugin for AnvilPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ChunkLoadEvent>()
            .add_event::<ChunkUnloadEvent>()
            .add_systems(PreUpdate, remove_unviewed_chunks)
            .add_systems(
                PostUpdate,
                (init_anvil, update_client_views, send_recv_chunks)
                    .chain()
                    .before(UpdateLayersPreClientSet),
            );
    }
}

fn init_anvil(mut query: Query<&mut AnvilLevel, (Added<AnvilLevel>, With<ChunkLayer>)>) {
    for mut level in &mut query {
        if let Some(state) = level.worker_state.take() {
            thread::spawn(move || anvil_worker(state));
        }
    }
}

/// Removes all chunks no longer viewed by clients.
///
/// This needs to run in `PreUpdate` where the chunk viewer counts have been
/// updated from the previous tick.
fn remove_unviewed_chunks(
    mut chunk_layers: Query<(Entity, &mut ChunkLayer, &AnvilLevel)>,
    mut unload_events: EventWriter<ChunkUnloadEvent>,
) {
    for (entity, mut layer, anvil) in &mut chunk_layers {
        layer.retain_chunks(|pos, chunk| {
            if chunk.viewer_count_mut() > 0 || anvil.ignored_chunks.contains(&pos) {
                true
            } else {
                unload_events.send(ChunkUnloadEvent {
                    chunk_layer: entity,
                    pos,
                });
                false
            }
        });
    }
}

fn update_client_views(
    clients: Query<(&EntityLayerId, Ref<OldEntityLayerId>, View, OldView), With<Client>>,
    mut chunk_layers: Query<(&ChunkLayer, &mut AnvilLevel)>,
) {
    for (loc, old_loc, view, old_view) in &clients {
        let view = view.get();
        let old_view = old_view.get();

        if loc != &*old_loc || view != old_view || old_loc.is_added() {
            let Ok((layer, mut anvil)) = chunk_layers.get_mut(loc.0) else {
                continue;
            };

            let queue_pos = |pos| {
                if !anvil.ignored_chunks.contains(&pos) && layer.chunk(pos).is_none() {
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
    mut layers: Query<(Entity, &mut ChunkLayer, &mut AnvilLevel)>,
    mut to_send: Local<Vec<(Priority, ChunkPos)>>,
    mut load_events: EventWriter<ChunkLoadEvent>,
) {
    for (entity, mut layer, anvil) in &mut layers {
        let anvil = anvil.into_inner();

        // Insert the chunks that are finished loading into the chunk layer and send
        // load events.
        for (pos, res) in anvil.receiver.drain() {
            anvil.pending.remove(&pos);

            let status = match res {
                Ok(Some(ParsedChunk { chunk, timestamp })) => {
                    layer.insert_chunk(pos, chunk);
                    ChunkLoadStatus::Success { timestamp }
                }
                Ok(None) => ChunkLoadStatus::Empty,
                Err(e) => ChunkLoadStatus::Failed(e),
            };

            load_events.send(ChunkLoadEvent {
                chunk_layer: entity,
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
        let res = state
            .dimension_folder
            .get_chunk(pos)
            .map_err(anyhow::Error::from);

        let _ = state.sender.send((pos, res));
    }
}

/// An event sent by `valence_anvil` after an attempt to load a chunk is made.
#[derive(Event, Debug)]
pub struct ChunkLoadEvent {
    /// The [`ChunkLayer`] where the chunk is located.
    pub chunk_layer: Entity,
    /// The position of the chunk in the layer.
    pub pos: ChunkPos,
    pub status: ChunkLoadStatus,
}

#[derive(Debug)]
pub enum ChunkLoadStatus {
    /// A new chunk was successfully loaded and inserted into the layer.
    Success {
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

/// An event sent by `valence_anvil` when a chunk is unloaded from an layer.
#[derive(Event, Debug)]
pub struct ChunkUnloadEvent {
    /// The [`ChunkLayer`] where the chunk was unloaded.
    pub chunk_layer: Entity,
    /// The position of the chunk that was unloaded.
    pub pos: ChunkPos,
}
