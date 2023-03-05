use std::path::PathBuf;

use clap::Parser;
use valence::bevy_app::AppExit;
use valence::client::despawn_disconnected_clients;
use valence::client::event::default_event_handler;
use valence::prelude::*;
use valence_schem::Schematic;

const SPAWN_POS: BlockPos = BlockPos::new(0, 256, 0);

#[derive(Parser)]
#[clap(author, version, about)]
struct Cli {
    /// The path to a Sponge Schematic.
    path: PathBuf,
}

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_startup_system(setup)
        .add_systems((
            default_event_handler.in_schedule(EventLoopSchedule),
            init_clients,
            despawn_disconnected_clients,
        ))
        .add_systems(PlayerList::default_systems())
        .run();
}

fn setup(mut commands: Commands, server: Res<Server>, mut exit: EventWriter<AppExit>) {
    let Cli { path } = Cli::parse();

    if !path.exists() {
        eprintln!("File `{}` does not exist. Exiting.", path.display());
        exit.send_default();
    } else if !path.is_file() {
        eprintln!("`{}` is not a file. Exiting.", path.display());
        exit.send_default();
    }

    let mut instance = server.new_instance(DimensionId::default());

    match Schematic::load(path) {
        Ok(schem) => {
            schem.paste(&mut instance, SPAWN_POS, |_| BiomeId::default());
        }
        Err(err) => {
            eprintln!("Error loading schematic: {err}");
            exit.send_default();
        }
    }

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
    mut commands: Commands,
) {
    for mut client in &mut clients {
        let instance = instances.single();

        client.set_flat(true);
        client.set_game_mode(GameMode::Creative);
        client.set_position([
            SPAWN_POS.x as f64 + 0.5,
            SPAWN_POS.y as f64,
            SPAWN_POS.z as f64 + 0.5,
        ]);
        client.set_instance(instance);

        commands.spawn(McEntity::with_uuid(
            EntityKind::Player,
            instance,
            client.uuid(),
        ));
    }
}
