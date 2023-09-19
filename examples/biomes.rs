#![allow(clippy::type_complexity)]

use rand::seq::IteratorRandom;
use rand::Rng;
use valence::prelude::*;
use valence::registry::biome::BiomeEffects;
use valence_server::dimension_layer::DimensionInfo;
use valence_server::BiomePos;

const SPAWN_Y: i32 = 0;
const SIZE: i32 = 5;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (init_clients, set_biomes))
        .run();
}

fn setup(
    mut commands: Commands,
    dimensions: Res<DimensionTypeRegistry>,
    mut biomes: ResMut<BiomeRegistry>,
    server: Res<Server>,
) {
    let colors = [
        0xeb4034, 0xffffff, 0xe3d810, 0x1fdbde, 0x1121d1, 0xe60ed7, 0xe68f0e, 0x840ee6, 0x0ee640,
    ];

    biomes.clear();

    // Client will be sad if you don't have a "plains" biome.
    biomes.insert(ident!("plains"), Biome::default());

    for color in colors {
        let name = Ident::new(format!("biome_{color:x}")).unwrap();

        let biome = Biome {
            effects: BiomeEffects {
                grass_color: Some(color),
                ..Default::default()
            },
            ..Default::default()
        };

        biomes.insert(name, biome);
    }

    let mut layer = CombinedLayerBundle::new(Default::default(), &dimensions, &biomes, &server);

    for z in -SIZE..SIZE {
        for x in -SIZE..SIZE {
            layer.chunk_index.insert([x, z], Chunk::new());
        }
    }

    for x in -SIZE * 16..SIZE * 16 {
        for z in -SIZE * 16..SIZE * 16 {
            layer
                .chunk
                .set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(layer);
}

fn set_biomes(mut layers: Query<&mut ChunkLayer>, biomes: Res<BiomeRegistry>) {
    let mut layer = layers.single_mut();

    let mut rng = rand::thread_rng();

    for _ in 0..10 {
        let x = rng.gen_range(-SIZE * 4..SIZE * 4);
        let z = rng.gen_range(-SIZE * 4..SIZE * 4);

        let biome = biomes
            .iter()
            .choose(&mut rng)
            .map(|(biome, _, _)| biome)
            .unwrap_or_default();

        layer.set_biome(BiomePos::new(x, SPAWN_Y / 4, z), biome);
    }
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
