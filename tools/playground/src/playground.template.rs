use valence::client::despawn_disconnected_clients;
use valence::log::LogPlugin;
use valence::network::ConnectionMode;
use valence::prelude::*;

#[allow(unused_imports)]
use crate::extras::*;

const SPAWN_Y: i32 = 64;

pub fn build_app(app: &mut App) {
    app.insert_resource(NetworkSettings {
        connection_mode: ConnectionMode::Offline,
        ..Default::default()
    })
    .add_plugins(DefaultPlugins.build().disable::<LogPlugin>())
    .add_systems(Startup, setup)
    .add_systems(EventLoopUpdate, toggle_gamemode_on_sneak)
    .add_systems(Update, (init_clients, despawn_disconnected_clients))
    .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    biomes: Res<BiomeRegistry>,
    dimensions: Res<DimensionTypeRegistry>,
) {
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<(&mut Location, &mut Position), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut loc, mut pos) in &mut clients {
        loc.0 = instances.single();
        pos.set([0.5, SPAWN_Y as f64 + 1.0, 0.5]);
    }
}

// Add more systems here!
