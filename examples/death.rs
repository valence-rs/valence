#![allow(clippy::type_complexity)]

use valence::prelude::*;
use valence::status::RequestRespawnEvent;
use valence_server::dimension_layer::DimensionInfo;

const SPAWN_Y: i32 = 64;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (init_clients, squat_and_die, necromancy))
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    for block in [
        BlockState::GRASS_BLOCK,
        BlockState::DEEPSLATE,
        BlockState::MAGMA_BLOCK,
    ] {
        let mut layer = CombinedLayerBundle::new(Default::default(), &dimensions, &biomes, &server);

        for z in -5..5 {
            for x in -5..5 {
                layer.chunk_index.insert([x, z], Chunk::new());
            }
        }

        for z in -25..25 {
            for x in -25..25 {
                layer.chunk_index.set_block([x, SPAWN_Y, z], block);
            }
        }

        commands.spawn(layer);
    }
}

fn init_clients(
    mut clients: Query<
        (
            &mut Client,
            &mut LayerId,
            &mut VisibleLayers,
            &mut Position,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, With<DimensionInfo>>,
) {
    for (mut client, mut layer_id, mut visible_layers, mut pos, mut game_mode) in &mut clients {
        let layer = layers.iter().next().unwrap();

        layer_id.0 = layer;
        visible_layers.0.insert(layer);
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        *game_mode = GameMode::Creative;

        client.send_chat_message(
            "Welcome to Valence! Sneak to die in the game (but not in real life).".italic(),
        );
    }
}

fn squat_and_die(mut clients: Query<&mut Client>, mut events: EventReader<SneakEvent>) {
    for event in events.iter() {
        if event.state == SneakState::Start {
            if let Ok(mut client) = clients.get_mut(event.client) {
                client.kill("Squatted too hard.");
            }
        }
    }
}

fn necromancy(
    mut clients: Query<(
        &mut LayerId,
        &mut VisibleLayers,
        &mut VisibleEntityLayers,
        &mut RespawnPosition,
    )>,
    mut events: EventReader<RequestRespawnEvent>,
    layers: Query<Entity, With<DimensionInfo>>,
) {
    for event in events.iter() {
        if let Ok((mut layer_id, mut visible_layers, mut respawn_pos)) =
            clients.get_mut(event.client)
        {
            respawn_pos.pos = BlockPos::new(0, SPAWN_Y, 0);

            // make the client respawn in another chunk layer.

            let idx = layers.iter().position(|l| l == layer_id.0).unwrap();
            let count = layers.iter().len();
            let layer = layers.into_iter().nth((idx + 1) % count).unwrap();

            layer_id.0 = layer;
            visible_chunk_layer.0 = layer;
            visible_entity_layers.0.clear();
            visible_entity_layers.0.insert(layer);
        }
    }
}
