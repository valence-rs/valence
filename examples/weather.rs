use std::f64::consts::TAU;

use valence::prelude::*;
use valence::weather::{Rain, Thunder, WeatherBundle};
use valence_server::dimension_layer::DimensionInfo;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (init_clients, change_weather))
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let mut layer = CombinedLayerBundle::new(Default::default(), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            layer.chunk_index.insert([x, z], Chunk::new());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            layer
                .chunk_index
                .set_block([x, 64, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn((layer, WeatherBundle::default()));
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
        visible_layers.insert(layer);
        pos.set([0.0, 65.0, 0.0]);
        *game_mode = GameMode::Creative;
    }
}

fn change_weather(
    mut layers: Query<(&mut Rain, &mut Thunder), With<DimensionInfo>>,
    server: Res<Server>,
) {
    let period = 5.0;

    let level = ((server.current_tick() as f64 / 20.0 * TAU / period).sin() + 1.0) / 2.0;

    for (mut rain, mut thunder) in &mut layers {
        rain.0 = level as f32;
        thunder.0 = level as f32;
    }
}
