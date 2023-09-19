use valence::log::LogPlugin;
use valence::network::ConnectionMode;
use valence::prelude::*;
use valence_server::dimension_layer::DimensionInfo;

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
    .add_systems(Update, init_clients)
    .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    biomes: Res<BiomeRegistry>,
    dimensions: Res<DimensionTypeRegistry>,
) {
    let mut layer = CombinedLayerBundle::new(Default::default(), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            layer.chunk_index.insert([x, z], UnloadedChunk::new());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            layer
                .chunk
                .set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(layer);
}

fn init_clients(
    mut clients: Query<
        (
            &mut LayerId,
            &mut VisibleLayers,
            &mut Position,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, With<DimensionInfo>>,
) {
    for (mut layer_id, mut visible_layers, mut pos, mut game_mode) in &mut clients {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_layers.0.insert(layer);
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        *game_mode = GameMode::Creative;
    }
}

// Add more systems here!
