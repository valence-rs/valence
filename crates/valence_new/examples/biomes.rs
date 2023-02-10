use valence_new::client::despawn_disconnected_clients;
use valence_new::client::event::default_event_handler;
use valence_new::prelude::*;

const SPAWN_Y: i32 = 0;
const BIOME_COUNT: usize = 10;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(
            ServerPlugin::new(()).with_biomes(
                (1..BIOME_COUNT)
                    .map(|i| {
                        let color = (0xffffff / BIOME_COUNT * i) as u32;
                        Biome {
                            name: ident!("valence:test_biome_{i}"),
                            sky_color: color,
                            water_fog_color: color,
                            fog_color: color,
                            water_color: color,
                            foliage_color: Some(color),
                            grass_color: Some(color),
                            ..Default::default()
                        }
                    })
                    .chain(std::iter::once(Biome {
                        name: ident!("plains"),
                        ..Default::default()
                    }))
                    .collect::<Vec<_>>(),
            ),
        )
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system_set(PlayerList::default_system_set())
        .run();
}

fn setup(world: &mut World) {
    let server = world.resource::<Server>();
    let mut instance = server.new_instance(DimensionId::default());

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
                        let biome_id = server
                            .biomes()
                            .nth((cx + cz * 4 + cy * 4 * 4) % BIOME_COUNT)
                            .unwrap()
                            .0;
                        chunk.set_biome(cx, cy, cz, biome_id);
                    }
                }
            }
            instance.insert_chunk([x, z], chunk);
        }
    }

    world.spawn(instance);
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for mut client in &mut clients {
        client.set_position([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        client.set_respawn_screen(true);
        client.set_instance(instances.single());
        client.set_game_mode(GameMode::Creative);
        client.send_message("Welcome to Valence!".italic());
    }
}
