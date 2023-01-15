use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ShouldRun;
use valence_new::client::event::default_event_handler;
use valence_new::client::Client;
use valence_new::config::Config;
use valence_new::dimension::DimensionId;
use valence_new::instance::{Chunk, Instance};
use valence_new::protocol::types::GameMode;
use valence_new::server::Server;
use valence_protocol::block::BlockState;

#[derive(Resource)]
struct GameState {
    instance: Entity,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    valence_new::run_server(
        Config::default(),
        SystemStage::parallel()
            .with_system(setup.with_run_criteria(ShouldRun::once))
            .with_system(init_clients)
            .with_system(default_event_handler())
            .with_system(tick),
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
            let mut chunk = Chunk::new(24);
            for z in 0..16 {
                for x in 0..16 {
                    chunk.set_block_state(x, 10, z, BlockState::STONE);
                }
            }

            instance.insert_chunk([x, z], chunk);
        }
    }

    let id = world.spawn(instance).id();
    world.insert_resource(GameState { instance: id })
}

fn init_clients(mut clients: Query<&mut Client, Added<Client>>, state: Res<GameState>) {
    for mut client in &mut clients {
        client.set_position([0.0, 32.0, 0.0]);
        client.set_instance(state.instance);
        client.set_game_mode(GameMode::Creative);
        client.set_view_distance(20);
    }
}

fn tick(state: Res<GameState>, server: Res<Server>, mut instances: Query<&mut Instance>) {
    if server.current_tick() % 20 == 0 {
        let mut instance = instances.get_mut(state.instance).unwrap();

        let y = ((10 + server.current_tick() / 20) % 120) as usize;

        instance
            .chunk_mut([0, 0])
            .unwrap()
            .set_block_state(0, y, 0, BlockState::MAGMA_BLOCK);
    }
}
