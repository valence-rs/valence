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
    let layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);
    let mut level = AnvilLevel::new(&cli.path, &biomes);

    // Force a 16x16 area of chunks around the origin to be loaded at all times.
    // This is similar to "spawn chunks" in vanilla. This isn't necessary for the
    // example to function, but it's done to demonstrate that it's possible.
    for z in -8..8 {
        for x in -8..8 {
            let pos = ChunkPos::new(x, z);

            level.ignored_chunks.insert(pos);
            level.force_chunk_load(pos);
        }
    }

    commands.spawn((layer, level));
}

fn init_clients(
    mut clients: Query<
        (
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, With<ChunkLayer>>,
) {
    for (
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut game_mode,
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set(SPAWN_POS);
        *game_mode = GameMode::Spectator;
    }
}

fn handle_chunk_loads(
    mut events: EventReader<ChunkLoadEvent>,
    mut layers: Query<&mut ChunkLayer, With<AnvilLevel>>,
) {
    let mut layer = layers.single_mut();

    for event in events.iter() {
        match &event.status {
            ChunkLoadStatus::Success { .. } => {
                // The chunk was inserted into the world. Nothing for us to do.
            }
            ChunkLoadStatus::Empty => {
                // There's no chunk here so let's insert an empty chunk. If we were doing
                // terrain generation we would prepare that here.
                layer.insert_chunk(event.pos, UnloadedChunk::new());
            }
            ChunkLoadStatus::Failed(e) => {
                // Something went wrong.
                let errmsg = format!(
                    "failed to load chunk at ({}, {}): {e:#}",
                    event.pos.x, event.pos.z
                );

                eprintln!("{errmsg}");
                layer.send_chat_message(errmsg.color(Color::RED));

                layer.insert_chunk(event.pos, UnloadedChunk::new());
            }
        }
    }
}

// Display the number of loaded chunks in the action bar of all clients.
fn display_loaded_chunk_count(mut layers: Query<&mut ChunkLayer>, mut last_count: Local<usize>) {
    let mut layer = layers.single_mut();

    let cnt = layer.chunks().count();

    if *last_count != cnt {
        *last_count = cnt;
        layer.send_action_bar_message("Chunk Count: ".into_text() + cnt.color(Color::LIGHT_PURPLE));
    }
}
