use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ShouldRun;
use valence_new::client::event::default_event_handler;
use valence_new::client::{despawn_disconnected_clients, Client};
use valence_new::config::Config;
use valence_new::dimension::DimensionId;
use valence_new::instance::{Chunk, Instance};
use valence_new::protocol::types::GameMode;
use valence_new::server::Server;
use valence_protocol::block::BlockState;

const SPAWN_Y: i32 = 64;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    valence_new::run_server(
        Config::default(),
        SystemStage::parallel()
            .with_system(setup.with_run_criteria(ShouldRun::once))
            .with_system(init_clients)
            .with_system(tick)
            .with_system(default_event_handler())
            .with_system(despawn_disconnected_clients)
            .with_system(teleport_oob_clients),
        (),
    )
}

fn setup(world: &mut World) {
    let mut instance = world
        .resource::<Server>()
        .new_instance(DimensionId::default());

    // Create spawn platform.
    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block_state([x, SPAWN_Y, z], BlockState::STONE);
        }
    }

    world.spawn(instance);
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    let instance = instances.get_single().unwrap();

    for mut client in &mut clients {
        client.set_position([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        client.set_instance(instance);
        client.set_game_mode(GameMode::Creative);
    }
}

fn tick(server: Res<Server>, mut instances: Query<&mut Instance>) {
    if server.current_tick() % 20 == 0 {
        let mut instance = instances.get_single_mut().unwrap();

        let y = (SPAWN_Y + server.current_tick() as i32 / 20) % 120;

        instance.set_block_state([5, y, 0], BlockState::MAGMA_BLOCK);

        if server.current_tick() % 40 == 0 {
            instance.set_block_state([6, y, 0], BlockState::LIME_WOOL);
        }
    }
}

fn teleport_oob_clients(mut clients: Query<&mut Client>) {
    for mut client in &mut clients {
        if client.position().y <= 0.0 {
            client.set_position([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        }
    }
}
