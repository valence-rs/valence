#![allow(clippy::type_complexity)]

use std::time::Instant;

use bevy_app::App;
use valence::client::despawn_disconnected_clients;
use valence::inventory::HeldItem;
use valence::message::SendMessage;
use valence::prelude::*;
use valence_world_time::{LinearTimeTicking, WorldTime, WorldTimeBundle};

const SPAWN_Y: i32 = 64;

fn main() {
    App::new()
        .insert_resource(LastTickTimestamp { time: Instant::now() })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                despawn_disconnected_clients,
                init_clients,
                show_time_info,
                change_time,
            ),
        )
        .run();
}


#[derive(Resource)]
struct LastTickTimestamp {
    time: Instant
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    biomes: Res<BiomeRegistry>,
    dimensions: Res<DimensionTypeRegistry>,
) {
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            layer
                .chunk
                .set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    let mut wb = WorldTimeBundle::default();
    wb.interval.0 = 200;

    commands.spawn((layer, wb));
}

fn show_time_info(
    layers: Query<(&WorldTime, &LinearTimeTicking)>,
    mut clients: Query<&mut Client>,
    server: Res<Server>,
    mut lt: ResMut<LastTickTimestamp>
) {
    let layer = layers.single();
    for mut c in &mut clients {
        let mspt = lt.time.elapsed().as_millis();
        lt.time = Instant::now();

        let msg = format!(
            "Server {} | mspt: {} | Time: {} | interval: {} | rate: {}",
            server.current_tick(),
            mspt,
            layer.0.time_of_day,
            layer.1.interval,
            layer.1.rate
        );
        c.send_action_bar_message(msg);
    }
}

fn change_time(
    mut event: EventReader<DiggingEvent>,
    client: Query<(&Client, &HeldItem)>,
    mut time: Query<&mut LinearTimeTicking>,
) {
    let mut ticker = time.single_mut();
    for e in &mut event {
        if let Ok((_, hi)) = client.get(e.client) {
            match hi.slot() {
                36 => ticker.rate += 1,
                37 => ticker.rate -= 1,
                38 => ticker.interval += 1,
                39 => ticker.interval -= 1,
                _ => (),
            };
        }
    }
}

fn init_clients(
    mut clients: Query<
        (
            &mut Client,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut GameMode,
            &mut Inventory,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        mut client,
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut game_mode,
        mut inv,
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        *game_mode = GameMode::Creative;

        client.send_chat_message(
            "
        Touch grass (left click) to control time!
        - Diamond: increase rate
        - Dirt: decrease rate
        - Clock: increase interval
        - Compass: decrease interval

        Have fun!
        ",
        );

        inv.set_slot(36, ItemStack::new(ItemKind::Diamond, 1, None));
        inv.set_slot(37, ItemStack::new(ItemKind::Dirt, 1, None));
        inv.set_slot(38, ItemStack::new(ItemKind::Clock, 1, None));
        inv.set_slot(39, ItemStack::new(ItemKind::Compass, 1, None));
    }
}
