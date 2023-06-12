use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::path::PathBuf;
use std::thread;

use clap::Parser;
use flume::{Receiver, Sender};
use tracing::warn;
use valence::anvil::{AnvilChunk, AnvilWorld};
use valence::prelude::*;

const SPAWN_POS: DVec3 = DVec3::new(0.0, 256.0, 0.0);
const SECTION_COUNT: usize = 24;

#[derive(Parser)]
#[clap(author, version, about)]
struct Cli {
    /// The path to a Minecraft world save containing a `region` subdirectory.
    path: PathBuf,
}

#[derive(Resource)]
struct GameState {
    /// Chunks that need to be generated. Chunks without a priority have already
    /// been sent to the anvil thread.
    pending: HashMap<ChunkPos, Option<Priority>>,
    sender: Sender<ChunkPos>,
    receiver: Receiver<(ChunkPos, Chunk)>,
}

/// The order in which chunks should be processed by anvil worker. Smaller
/// values are sent first.
type Priority = u64;

pub fn main() {
    tracing_subscriber::fmt().init();

    let cli = Cli::parse();
    let dir = cli.path;

    if !dir.exists() {
        eprintln!("Directory `{}` does not exist. Exiting.", dir.display());
        return;
    } else if !dir.is_dir() {
        eprintln!("`{}` is not a directory. Exiting.", dir.display());
        return;
    }

    let anvil = AnvilWorld::new(dir);

    let (finished_sender, finished_receiver) = flume::unbounded();
    let (pending_sender, pending_receiver) = flume::unbounded();

    // Process anvil chunks in a different thread to avoid blocking the main tick
    // loop.
    thread::spawn(move || anvil_worker(pending_receiver, finished_sender, anvil));

    let game_state = GameState {
        pending: HashMap::new(),
        sender: pending_sender,
        receiver: finished_receiver,
    };

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(game_state)
        .add_startup_system(setup)
        .add_systems(
            (
                init_clients,
                remove_unviewed_chunks,
                update_client_views,
                send_recv_chunks,
            )
                .chain(),
        )
        .add_system(despawn_disconnected_clients)
        .run();
}

fn setup(
    mut commands: Commands,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    server: Res<Server>,
) {
    let instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);
    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<(&mut Location, &mut Position, &mut GameMode, &mut IsFlat), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut loc, mut pos, mut game_mode, mut is_flat) in &mut clients {
        loc.0 = instances.single();
        pos.set(SPAWN_POS);
        *game_mode = GameMode::Creative;
        is_flat.0 = true;
    }
}

fn remove_unviewed_chunks(mut instances: Query<&mut Instance>) {
    instances
        .single_mut()
        .retain_chunks(|_, chunk| chunk.is_viewed_mut());
}

fn update_client_views(
    mut instances: Query<&mut Instance>,
    mut clients: Query<(&mut Client, View, OldView)>,
    mut state: ResMut<GameState>,
) {
    let instance = instances.single_mut();

    for (client, view, old_view) in &mut clients {
        let view = view.get();
        let queue_pos = |pos| {
            if instance.chunk(pos).is_none() {
                match state.pending.entry(pos) {
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
        if client.is_added() {
            view.iter().for_each(queue_pos);
        } else {
            let old_view = old_view.get();
            if old_view != view {
                view.diff(old_view).for_each(queue_pos);
            }
        }
    }
}

fn send_recv_chunks(mut instances: Query<&mut Instance>, state: ResMut<GameState>) {
    let mut instance = instances.single_mut();
    let state = state.into_inner();

    // Insert the chunks that are finished loading into the instance.
    for (pos, chunk) in state.receiver.drain() {
        instance.insert_chunk(pos, chunk);
        assert!(state.pending.remove(&pos).is_some());
    }

    // Collect all the new chunks that need to be loaded this tick.
    let mut to_send = vec![];

    for (pos, priority) in &mut state.pending {
        if let Some(pri) = priority.take() {
            to_send.push((pri, pos));
        }
    }

    // Sort chunks by ascending priority.
    to_send.sort_unstable_by_key(|(pri, _)| *pri);

    // Send the sorted chunks to be loaded.
    for (_, pos) in to_send {
        let _ = state.sender.try_send(*pos);
    }
}

fn anvil_worker(
    receiver: Receiver<ChunkPos>,
    sender: Sender<(ChunkPos, Chunk)>,
    mut world: AnvilWorld,
) {
    while let Ok(pos) = receiver.recv() {
        match get_chunk(pos, &mut world) {
            Ok(chunk) => {
                if let Some(chunk) = chunk {
                    let _ = sender.try_send((pos, chunk));
                }
            }
            Err(e) => warn!("Failed to get chunk at ({}, {}): {e:#}.", pos.x, pos.z),
        }
    }
}

fn get_chunk(pos: ChunkPos, world: &mut AnvilWorld) -> anyhow::Result<Option<Chunk>> {
    let Some(AnvilChunk { data, .. }) = world.read_chunk(pos.x, pos.z)? else {
        return Ok(None)
    };

    let mut chunk = Chunk::new(SECTION_COUNT);

    valence_anvil::to_valence(&data, &mut chunk, 4, |_| BiomeId::default())?;

    Ok(Some(chunk))
}
