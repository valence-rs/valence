#![allow(clippy::type_complexity)]

use valence::client::{default_event_handler, despawn_disconnected_clients};
use valence::prelude::*;

const SPAWN_Y: i32 = 0;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_startup_system(setup)
        .add_systems((
            default_event_handler.in_schedule(EventLoopSchedule),
            init_clients,
            despawn_disconnected_clients,
        ))
        .add_systems(PlayerList::default_systems())
        .run();
}

fn setup(
    mut commands: Commands,
    dimensions: Query<&DimensionType>,
    biomes: Query<&Biome>,
    biome_reg: Res<BiomeRegistry>,
    server: Res<Server>,
) {
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

    let biome_count = biome_reg.iter().count();

    for z in -5..5 {
        for x in -5..5 {
            let mut chunk = Chunk::new(4);
            // Set chunk blocks
            for z in 0..16 {
                for x in 0..16 {
                    chunk.set_block_state(x, 63, z, BlockState::GRASS_BLOCK);
                }
            }

            // Set the biomes of the chunk to a 4x4x4 grid of biomes
            for cz in 0..4 {
                for cx in 0..4 {
                    let height = chunk.section_count() * 16;
                    for cy in 0..height / 4 {
                        let biome_id = biome_reg
                            .iter()
                            .nth((cx + cz * 4 + cy * 4 * 4) % biome_count)
                            .unwrap()
                            .0;

                        chunk.set_biome(cx, cy, cz, biome_id);
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
