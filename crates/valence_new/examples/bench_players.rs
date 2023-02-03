use std::time::Instant;

use bevy_app::{App, CoreStage};
use valence_new::client::despawn_disconnected_clients;
use valence_new::client::event::default_event_handler;
use valence_new::instance::{Chunk, Instance};
use valence_new::player_list::{
    add_new_clients_to_player_list, remove_disconnected_clients_from_player_list,
};
use valence_new::prelude::*;

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
        .add_system(add_new_clients_to_player_list)
        .add_system(remove_disconnected_clients_from_player_list)
        .run();
}

fn record_tick_start_time(world: &mut World) {
    world
        .get_resource_or_insert_with(|| TickStart(Instant::now()))
        .0 = Instant::now();
}

fn print_tick_time(server: Res<Server>, time: Res<TickStart>, clients: Query<(), With<Client>>) {
    let tick = server.current_tick();
    if tick % (server.tps() / 2) == 0 {
        let client_count = clients.iter().count();

        let millis = time.0.elapsed().as_secs_f32() * 1000.0;
        println!("Tick={tick}, MSPT={millis:.04}ms, Clients={client_count}");
    }
}

fn setup(world: &mut World) {
    let mut instance = world
        .resource::<Server>()
        .new_instance(DimensionId::default());

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    for z in -50..50 {
        for x in -50..50 {
            instance.set_block_state([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    world.spawn(instance);
}

fn init_clients(
    mut clients: Query<(Entity, &mut Client), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
    mut commands: Commands,
) {
    let instance = instances.get_single().unwrap();

    for (client_entity, mut client) in &mut clients {
        client.set_position([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        client.set_instance(instance);
        client.set_game_mode(GameMode::Creative);

        let player_entity = McEntity::with_uuid(EntityKind::Player, instance, client.uuid());

        commands.entity(client_entity).insert(player_entity);
    }
}
