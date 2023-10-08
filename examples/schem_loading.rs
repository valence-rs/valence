use std::path::PathBuf;

use clap::Parser;
use valence::prelude::*;
use valence_schem::Schematic;

const SPAWN_POS: BlockPos = BlockPos::new(0, 256, 0);

#[derive(Parser)]
#[clap(author, version, about)]
struct Cli {
    /// The path to a Sponge Schematic.
    path: PathBuf,
}

#[derive(Resource)]
struct SchemRes(Schematic);

pub fn main() {
    let Cli { path } = Cli::parse();
    if !path.exists() {
        eprintln!("File `{}` does not exist. Exiting.", path.display());
        return;
    } else if !path.is_file() {
        eprintln!("`{}` is not a file. Exiting.", path.display());
        return;
    }
    let schem = match Schematic::load(path) {
        Ok(schem) => schem,
        Err(err) => {
            eprintln!("Error loading schematic: {err}");
            return;
        }
    };

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(SchemRes(schem))
        .add_systems(Startup, setup)
        .add_systems(Update, (init_clients, despawn_disconnected_clients))
        .run();
}

fn setup(
    mut commands: Commands,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    server: Res<Server>,
    schem: Res<SchemRes>,
) {
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);
    schem
        .0
        .paste(&mut layer.chunk, SPAWN_POS, |_| BiomeId::default());
    commands.spawn(layer);
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
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
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
        pos.set([
            SPAWN_POS.x as f64 + 0.5,
            SPAWN_POS.y as f64,
            SPAWN_POS.z as f64 + 0.5,
        ]);
        *game_mode = GameMode::Creative;
    }
}
