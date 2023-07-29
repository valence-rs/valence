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
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);
    schem
        .0
        .paste(&mut instance, SPAWN_POS, |_| BiomeId::default());
    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<(&mut Location, &mut Position, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut loc, mut pos, mut game_mode) in &mut clients {
        *game_mode = GameMode::Creative;
        pos.set([
            SPAWN_POS.x as f64 + 0.5,
            SPAWN_POS.y as f64,
            SPAWN_POS.z as f64 + 0.5,
        ]);
        loc.0 = instances.single();
    }
}
