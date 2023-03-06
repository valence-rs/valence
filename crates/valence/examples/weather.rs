use valence::client::despawn_disconnected_clients;
use valence::client::event::default_event_handler;
use valence::instance::weather::Weather;
use valence::prelude::*;

pub fn main() {
    App::new()
        .add_plugin(ServerPlugin::new(()).with_connection_mode(ConnectionMode::Offline))
        .add_system(default_event_handler.in_schedule(EventLoopSchedule))
        .add_system(despawn_disconnected_clients)
        .add_system(init_clients)
        .add_startup_system(setup)
        .run();
}

const WORLD_SIZE: i32 = 8;
const CHUNK_SIZE: i32 = 16;
const SPAWN_Y: i32 = 64;

fn setup(mut commands: Commands, server: Res<Server>) {
    let mut instance = server.new_instance(DimensionId::default());

    let chunks = WORLD_SIZE;

    for z in -chunks..chunks {
        for x in -chunks..chunks {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    let blocks = CHUNK_SIZE * chunks / 2;

    for z in -blocks..blocks {
        for x in -blocks..blocks {
            instance.set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(instance).insert(Weather {
        rain: Some(1_f32),
        thunder: None,
    });
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    let instance = instances.get_single().unwrap();

    for mut client in &mut clients {
        client.set_position([0.5, SPAWN_Y as f64 + 1.0, 0.5]);
        client.set_instance(instance);
        client.set_game_mode(GameMode::Survival);
    }
}
