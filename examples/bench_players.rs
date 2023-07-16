#![allow(clippy::type_complexity)]

use std::time::Instant;

use valence::prelude::*;
use valence_client::{VisibleChunkLayer, VisibleEntityLayers};

const SPAWN_Y: i32 = 64;

#[derive(Resource)]
struct TickStart(Instant);

fn main() {
    App::new()
        .insert_resource(CoreSettings {
            compression_threshold: None,
            ..Default::default()
        })
        .insert_resource(NetworkSettings {
            connection_mode: ConnectionMode::Offline,
            max_connections: 50_000,
            max_players: 50_000,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(First, record_tick_start_time)
        .add_systems(Update, (init_clients, despawn_disconnected_clients))
        .add_systems(Last, print_tick_time)
        .run();
}

fn record_tick_start_time(mut commands: Commands) {
    commands.insert_resource(TickStart(Instant::now()));
}

fn print_tick_time(
    server: Res<Server>,
    settings: Res<CoreSettings>,
    time: Res<TickStart>,
    clients: Query<(), With<Client>>,
) {
    let tick = server.current_tick();
    if tick % (settings.tick_rate.get() as i64 / 2) == 0 {
        let client_count = clients.iter().len();

        let millis = time.0.elapsed().as_secs_f32() * 1000.0;
        println!("Tick={tick}, MSPT={millis:.04}ms, Clients={client_count}");
    }
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

    for z in -50..50 {
        for x in -50..50 {
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
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        *game_mode = GameMode::Creative;
    }
}
