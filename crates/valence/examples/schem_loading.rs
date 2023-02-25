use std::path::PathBuf;

use bevy_ecs::prelude::*;
use clap::Parser;
use glam::DVec3;
use valence::prelude::*;
use valence_schem::Schematic;

const SPAWN_POS: DVec3 = DVec3::new(0.0, 256.0, 0.0);

#[derive(Parser)]
#[clap(author, version, about)]
struct Cli {
    /// The path to a Sponge Schematic.
    path: PathBuf,
}

#[derive(Resource)]
struct SchemRes(Schematic);

pub fn main() {
    tracing_subscriber::fmt().init();

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
        .insert_resource(SchemRes(schem))
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .run();
}

fn setup(
    mut commands: Commands,
    dimensions: Query<&DimensionType>,
    biomes: Query<&Biome>,
    server: Res<Server>,
    schem: Res<SchemRes>,
) {
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

    schem.0.paste(&mut instance, BlockPos::new(0, 0, 0), |_| {
        BiomeId::default()
    });

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<(&mut Location, &mut Position, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut loc, mut pos, mut game_mode) in &mut clients {
        *game_mode = GameMode::Creative;
        loc.0 = instances.single();
        pos.set(SPAWN_POS);
    }
}
