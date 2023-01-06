use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ShouldRun;
use uuid::Uuid;
use valence_new::client::Client;
use valence_new::config::Config;
use valence_new::dimension::DimensionId;
use valence_new::instance::Instance;
use valence_new::protocol::types::GameMode;
use valence_new::server::Server;

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
            .with_system(init_clients),
        (),
    )
}

fn setup(world: &mut World) {
    let instance = world
        .resource::<Server>()
        .new_instance(DimensionId::default());

    let id = world.spawn(instance).id();
    world.insert_resource(GameState { instance: id })
}

fn init_clients(mut clients: Query<&mut Client, Added<Client>>, state: Res<GameState>) {
    for mut client in &mut clients {
        client.set_instance(state.instance);
        client.set_game_mode(GameMode::Creative);
    }
}
