use std::path::PathBuf;

use clap::Parser;
use valence::prelude::*;
use valence_anvil::{AnvilLevel, ChunkLoadEvent, ChunkLoadStatus};

const SPAWN_POS: DVec3 = DVec3::new(0.0, 256.0, 0.0);

#[derive(Parser, Resource)]
#[clap(author, version, about)]
struct Cli {
    /// The path to a Minecraft world save containing a `region` subdirectory.
    path: PathBuf,
}

pub fn main() {
    tracing_subscriber::fmt().init();

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
        .add_startup_system(setup)
        .add_system(despawn_disconnected_clients)
        .add_systems((init_clients, handle_chunk_load_events).chain())
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
    let level = AnvilLevel::new(&cli.path, &biomes);

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

fn handle_chunk_load_events(
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
                // There's no chunk here so let's insert an empty chunk.
                inst.insert_chunk(event.pos, Chunk::new(0));
            }
            ChunkLoadStatus::Failed(e) => {
                // Something went wrong.
                eprintln!(
                    "failed to load chunk at ({}, {}): {e:#}",
                    event.pos.x, event.pos.z
                );
                inst.insert_chunk(event.pos, Chunk::new(0));
            }
        }
    }
}
