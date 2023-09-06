use std::f64::consts::TAU;

use valence::prelude::*;
use valence::weather::{Rain, Thunder, WeatherBundle};
use valence_server::nbt::{compound, List};

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (init_clients, despawn_disconnected_clients, change_weather),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            layer.chunk.set_block([x, 0, z], BlockState::GRASS_BLOCK);
        }
    }

    for z in 1..25 {
        for x in -10..10 {
            layer.chunk.set_block([x, z, z+5], BlockState::STONE);
        }
    }

    layer.chunk.set_block(
        [2,1,5],
        Block {
            state: BlockState::OAK_SIGN.set(PropName::Rotation, PropValue::_7),
            nbt: Some(compound! {
                "front_text" => compound! {
                    "messages" => List::String(vec![
                        "This stairway".into_text().into(),
                        "demonstrates the ".into_text().into(),
                        "MOTION_BLOCKING".into_text().into(),
                        "heightmap.".into_text().into(),
                    ]),
                }
            }),
        },
    );

    commands.spawn((layer, WeatherBundle::default()));
}

fn init_clients(
    mut clients: Query<
        (
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut game_mode,
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set([0.0, 1.0, 0.0]);
        *game_mode = GameMode::Creative;
    }
}

fn change_weather(
    mut layers: Query<(&mut Rain, &mut Thunder), With<ChunkLayer>>,
    server: Res<Server>,
) {
    let period = 10.0;

    let level = ((server.current_tick() as f64 / 20.0 * TAU / period).sin() + 1.0) / 2.0;

    for (mut rain, mut thunder) in &mut layers {
        rain.0 = level as f32;
        thunder.0 = level as f32;
    }
}
