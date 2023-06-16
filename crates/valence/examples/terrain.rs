#![allow(clippy::type_complexity)]

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::SystemTime;

use flume::{Receiver, Sender};
use noise::{NoiseFn, SuperSimplex};
use tracing::info;
use valence::prelude::*;

const SPAWN_POS: DVec3 = DVec3::new(0.0, 200.0, 0.0);
const SECTION_COUNT: usize = 24;

struct ChunkWorkerState {
    sender: Sender<(ChunkPos, Chunk)>,
    receiver: Receiver<ChunkPos>,
    // Noise functions
    density: SuperSimplex,
    hilly: SuperSimplex,
    stone: SuperSimplex,
    gravel: SuperSimplex,
    grass: SuperSimplex,
}

#[derive(Resource)]
struct GameState {
    /// Chunks that need to be generated. Chunks without a priority have already
    /// been sent to the thread pool.
    pending: HashMap<ChunkPos, Option<Priority>>,
    sender: Sender<ChunkPos>,
    receiver: Receiver<(ChunkPos, Chunk)>,
}

/// The order in which chunks should be processed by the thread pool. Smaller
/// values are sent first.
type Priority = u64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugins(DefaultPlugins)
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
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let seconds_per_day = 86_400;
    let seed = (SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        / seconds_per_day) as u32;

    info!("current seed: {seed}");

    let (finished_sender, finished_receiver) = flume::unbounded();
    let (pending_sender, pending_receiver) = flume::unbounded();

    let state = Arc::new(ChunkWorkerState {
        sender: finished_sender,
        receiver: pending_receiver,
        density: SuperSimplex::new(seed),
        hilly: SuperSimplex::new(seed.wrapping_add(1)),
        stone: SuperSimplex::new(seed.wrapping_add(2)),
        gravel: SuperSimplex::new(seed.wrapping_add(3)),
        grass: SuperSimplex::new(seed.wrapping_add(4)),
    });

    // Chunks are generated in a thread pool for parallelism and to avoid blocking
    // the main tick loop. You can use your thread pool of choice here (rayon,
    // bevy_tasks, etc). Only the standard library is used in the example for the
    // sake of simplicity.
    //
    // If your chunk generation algorithm is inexpensive then there's no need to do
    // this.
    for _ in 0..thread::available_parallelism().unwrap().get() {
        let state = state.clone();
        thread::spawn(move || chunk_worker(state));
    }

    commands.insert_resource(GameState {
        pending: HashMap::new(),
        sender: pending_sender,
        receiver: finished_receiver,
    });

    let instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<(&mut Location, &mut Position, &mut IsFlat, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut loc, mut pos, mut is_flat, mut game_mode) in &mut clients {
        loc.0 = instances.single();
        pos.set(SPAWN_POS);
        is_flat.0 = true;
        *game_mode = GameMode::Creative;
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

        // Queue all the new chunks in the view to be sent to the thread pool.
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

    // Insert the chunks that are finished generating into the instance.
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

fn chunk_worker(state: Arc<ChunkWorkerState>) {
    while let Ok(pos) = state.receiver.recv() {
        let mut chunk = Chunk::new(SECTION_COUNT);

        for offset_z in 0..16 {
            for offset_x in 0..16 {
                let x = offset_x as i32 + pos.x * 16;
                let z = offset_z as i32 + pos.z * 16;

                let mut in_terrain = false;
                let mut depth = 0;

                // Fill in the terrain column.
                for y in (0..chunk.section_count() as i32 * 16).rev() {
                    const WATER_HEIGHT: i32 = 55;

                    let p = DVec3::new(x as f64, y as f64, z as f64);

                    let block = if has_terrain_at(&state, p) {
                        let gravel_height = WATER_HEIGHT
                            - 1
                            - (fbm(&state.gravel, p / 10.0, 3, 2.0, 0.5) * 6.0).floor() as i32;

                        if in_terrain {
                            if depth > 0 {
                                depth -= 1;
                                if y < gravel_height {
                                    BlockState::GRAVEL
                                } else {
                                    BlockState::DIRT
                                }
                            } else {
                                BlockState::STONE
                            }
                        } else {
                            in_terrain = true;
                            let n = noise01(&state.stone, p / 15.0);

                            depth = (n * 5.0).round() as u32;

                            if y < gravel_height {
                                BlockState::GRAVEL
                            } else if y < WATER_HEIGHT - 1 {
                                BlockState::DIRT
                            } else {
                                BlockState::GRASS_BLOCK
                            }
                        }
                    } else {
                        in_terrain = false;
                        depth = 0;
                        if y < WATER_HEIGHT {
                            BlockState::WATER
                        } else {
                            BlockState::AIR
                        }
                    };

                    chunk.set_block_state(offset_x, y as usize, offset_z, block);
                }

                // Add grass on top of grass blocks.
                for y in (0..chunk.section_count() * 16).rev() {
                    if chunk.block_state(offset_x, y, offset_z).is_air()
                        && chunk.block_state(offset_x, y - 1, offset_z) == BlockState::GRASS_BLOCK
                    {
                        let p = DVec3::new(x as f64, y as f64, z as f64);
                        let density = fbm(&state.grass, p / 5.0, 4, 2.0, 0.7);

                        if density > 0.55 {
                            if density > 0.7
                                && chunk.block_state(offset_x, y + 1, offset_z).is_air()
                            {
                                let upper =
                                    BlockState::TALL_GRASS.set(PropName::Half, PropValue::Upper);
                                let lower =
                                    BlockState::TALL_GRASS.set(PropName::Half, PropValue::Lower);

                                chunk.set_block_state(offset_x, y + 1, offset_z, upper);
                                chunk.set_block_state(offset_x, y, offset_z, lower);
                            } else {
                                chunk.set_block_state(offset_x, y, offset_z, BlockState::GRASS);
                            }
                        }
                    }
                }
            }
        }

        let _ = state.sender.try_send((pos, chunk));
    }
}

fn has_terrain_at(state: &ChunkWorkerState, p: DVec3) -> bool {
    let hilly = lerp(0.1, 1.0, noise01(&state.hilly, p / 400.0)).powi(2);

    let lower = 15.0 + 100.0 * hilly;
    let upper = lower + 100.0 * hilly;

    if p.y <= lower {
        return true;
    } else if p.y >= upper {
        return false;
    }

    let density = 1.0 - lerpstep(lower, upper, p.y);

    let n = fbm(&state.density, p / 100.0, 4, 2.0, 0.5);

    n < density
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a * (1.0 - t) + b * t
}

fn lerpstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    if x <= edge0 {
        0.0
    } else if x >= edge1 {
        1.0
    } else {
        (x - edge0) / (edge1 - edge0)
    }
}

fn fbm(noise: &SuperSimplex, p: DVec3, octaves: u32, lacunarity: f64, persistence: f64) -> f64 {
    let mut freq = 1.0;
    let mut amp = 1.0;
    let mut amp_sum = 0.0;
    let mut sum = 0.0;

    for _ in 0..octaves {
        let n = noise01(noise, p * freq);
        sum += n * amp;
        amp_sum += amp;

        freq *= lacunarity;
        amp *= persistence;
    }

    // Scale the output to [0, 1]
    sum / amp_sum
}

fn noise01(noise: &SuperSimplex, p: DVec3) -> f64 {
    (noise.get(p.to_array()) + 1.0) / 2.0
}
