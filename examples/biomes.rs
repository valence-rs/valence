#![allow(clippy::type_complexity)]

use valence::prelude::*;

const SPAWN_Y: i32 = 0;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (init_clients, despawn_disconnected_clients))
        .run();
}

fn setup(
    mut commands: Commands,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    biome_reg: Res<BiomeRegistry>,
    server: Res<Server>,
) {
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

    let biome_count = biome_reg.iter().count() as u32;

    for z in -5..5 {
        for x in -5..5 {
            let mut chunk = UnloadedChunk::with_height(64);
            // Set chunk blocks
            for z in 0..16 {
                for x in 0..16 {
                    chunk.set_block_state(x, 63, z, BlockState::GRASS_BLOCK);
                }
            }

            // Set the biomes of the chunk to a 4x4x4 grid of biomes
            for bz in 0..4 {
                for bx in 0..4 {
                    for by in 0..chunk.height() / 4 {
                        let nth = (bx + bz * 4 + by * 4 * 4) % biome_count;

                        let biome_id = biome_reg.iter().nth(nth as usize).unwrap().0;

                        chunk.set_biome(bx, by, bz, biome_id);
                    }
                }
            }
            instance.insert_chunk([x, z], chunk);
        }
    }

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<(&mut Position, &mut Location, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut pos, mut loc, mut game_mode) in &mut clients {
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        loc.0 = instances.single();
        *game_mode = GameMode::Creative;
    }
}
