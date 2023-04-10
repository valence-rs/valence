#![allow(clippy::type_complexity)]

use std::time::Instant;

use valence::instance::{Chunk, Instance};
use valence::prelude::*;

const SPAWN_Y: i32 = 64;

#[derive(Resource)]
struct TickStart(Instant);

fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(
            ServerPlugin::new(())
                .with_connection_mode(ConnectionMode::Offline)
                .with_compression_threshold(None)
                .with_max_connections(50_000),
        )
        .add_startup_system(setup)
        .add_systems((
            record_tick_start_time.in_base_set(CoreSet::First),
            print_tick_time.in_base_set(CoreSet::Last),
            init_clients,
            despawn_disconnected_clients,
        ))
        .run();
}

fn record_tick_start_time(mut commands: Commands) {
    commands.insert_resource(TickStart(Instant::now()));
}

fn print_tick_time(server: Res<Server>, time: Res<TickStart>, clients: Query<(), With<Client>>) {
    let tick = server.current_tick();
    if tick % (server.tps() / 2) == 0 {
        let client_count = clients.iter().count();

        let millis = time.0.elapsed().as_secs_f32() * 1000.0;
        println!("Tick={tick}, MSPT={millis:.04}ms, Clients={client_count}");
    }
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Query<&DimensionType>,
    biomes: Query<&Biome>,
) {
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    for z in -50..50 {
        for x in -50..50 {
            instance.set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<(&mut Location, &mut Position, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut loc, mut pos, mut game_mode) in &mut clients {
        loc.0 = instances.single();
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        *game_mode = GameMode::Creative;
    }
}
