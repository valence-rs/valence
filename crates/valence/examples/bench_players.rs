#![allow(clippy::type_complexity)]

use std::time::Instant;

use valence::client::despawn_disconnected_clients;
use valence::client::default_event_handler;
use valence::entity::player::PlayerBundle;
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
            default_event_handler.in_schedule(EventLoopSchedule),
            record_tick_start_time.in_base_set(CoreSet::First),
            print_tick_time.in_base_set(CoreSet::Last),
            init_clients,
            despawn_disconnected_clients,
        ))
        .add_systems(PlayerList::default_systems())
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
    mut clients: Query<
        (
            Entity,
            &UniqueId,
            &mut Position,
            &mut Location,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    instances: Query<Entity, With<Instance>>,
    mut commands: Commands,
) {
    for (entity, uuid, mut pos, mut loc, mut game_mode) in &mut clients {
        pos.0 = [0.0, SPAWN_Y as f64 + 1.0, 0.0].into();
        loc.0 = instances.single();
        *game_mode = GameMode::Creative;

        commands.entity(entity).insert(PlayerBundle {
            location: *loc,
            uuid: *uuid,
            ..Default::default()
        });
    }
}
