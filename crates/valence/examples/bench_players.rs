use std::time::Instant;

use bevy_app::{App, CoreStage};
use valence::client::despawn_disconnected_clients;
use valence::client::event::default_event_handler;
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
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_system_to_stage(CoreStage::First, record_tick_start_time)
        .add_system_to_stage(CoreStage::Last, print_tick_time)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system_set(PlayerList::default_system_set())
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

fn setup(mut commands: Commands, server: Res<Server>) {
    let mut instance = server.new_instance(DimensionId::default());

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
    mut clients: Query<(Entity, &mut Client), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
    mut commands: Commands,
) {
    let instance = instances.single();

    for (client_entity, mut client) in &mut clients {
        client.set_position([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        client.set_instance(instance);
        client.set_game_mode(GameMode::Creative);

        let player_entity = McEntity::with_uuid(EntityKind::Player, instance, client.uuid());

        commands.entity(client_entity).insert(player_entity);
    }
}
