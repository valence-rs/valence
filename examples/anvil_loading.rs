use std::path::PathBuf;

use clap::Parser;
use valence::prelude::*;
use valence_anvil::{AnvilLevel, ChunkLoadEvent, ChunkLoadStatus};
use valence_client::message::SendMessage;

const SPAWN_POS: DVec3 = DVec3::new(0.0, 256.0, 0.0);

#[derive(Parser, Resource)]
#[clap(author, version, about)]
struct Cli {
    /// The path to a Minecraft world save containing a `region` subdirectory.
    path: PathBuf,
}

pub fn main() {
    let cli = Cli::parse();

    if !cli.path.exists() {
        eprintln!(
            "Directory `{}` does not exist. Exiting.",
            cli.path.display()
        );
        return;
    }

    if !cli.path.is_dir() {
        eprintln!("`{}` is not a directory. Exiting.", cli.path.display());
        return;
    }

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(cli)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                despawn_disconnected_clients,
                (init_clients, handle_chunk_loads).chain(),
                display_loaded_chunk_count,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    server: Res<Server>,
    cli: Res<Cli>,
) {
    let instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);
    let mut level = AnvilLevel::new(&cli.path, &biomes);

    // Force a 16x16 area of chunks around the origin to be loaded at all times,
    // similar to spawn chunks in vanilla. This isn't necessary, but it is done to
    // demonstrate that it is possible.
    for z in -8..8 {
        for x in -8..8 {
            let pos = ChunkPos::new(x, z);

            level.ignored_chunks.insert(pos);
            level.force_chunk_load(pos);
        }
    }

    commands.spawn((instance, level));
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

fn handle_chunk_loads(
    mut events: EventReader<ChunkLoadEvent>,
    mut instances: Query<&mut Instance, With<AnvilLevel>>,
) {
    let mut inst = instances.single_mut();

    for event in events.iter() {
        match &event.status {
            ChunkLoadStatus::Success { .. } => {
                // The chunk was inserted into the world. Nothing for us to do.
            }
            ChunkLoadStatus::Empty => {
                // There's no chunk here so let's insert an empty chunk. If we were doing
                // terrain generation we would prepare that here.
                inst.insert_chunk(event.pos, UnloadedChunk::new());
            }
            ChunkLoadStatus::Failed(e) => {
                // Something went wrong.
                eprintln!(
                    "failed to load chunk at ({}, {}): {e:#}",
                    event.pos.x, event.pos.z
                );
                inst.insert_chunk(event.pos, UnloadedChunk::new());
            }
        }
    }
}

// Display the number of loaded chunks in the action bar of all clients.
fn display_loaded_chunk_count(mut instances: Query<&mut Instance>, mut last_count: Local<usize>) {
    let mut inst = instances.single_mut();

    let cnt = inst.chunks().count();

    if *last_count != cnt {
        *last_count = cnt;
        inst.send_action_bar_message(
            "Chunk Count: ".into_text() + (cnt as i32).color(Color::LIGHT_PURPLE),
        );
    }
}
