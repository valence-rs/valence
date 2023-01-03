use bevy_ecs::prelude::*;
use valence_new::config::Config;
use valence_new::dimension::DimensionId;
use valence_new::instance::Instance;
use valence_new::server::Server;

#[derive(Resource)]
struct GameState {
    instance: Entity,
}

fn main() -> anyhow::Result<()> {
    let mut world = World::new();
    let instance = world.spawn(Instance::default()).id();

    world.insert_resource(GameState { instance });

    valence_new::run_server(
        Config::default().with_world(world),
        SystemStage::parallel(),
        (),
    )
}
