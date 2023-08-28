#![allow(clippy::type_complexity)]

use rand::Rng;
use valence::keepalive::Ping;
use valence::player_list::{DisplayName, PlayerListEntryBundle};
use valence::prelude::*;

const SPAWN_Y: i32 = 64;
const PLAYER_UUID_1: Uuid = Uuid::from_u128(1);
const PLAYER_UUID_2: Uuid = Uuid::from_u128(2);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                init_clients,
                override_display_name,
                update_player_list,
                despawn_disconnected_clients,
            ),
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
            layer.chunk.insert_chunk([x, z], Chunk::new());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            layer
                .chunk
                .set_block([x, SPAWN_Y, z], BlockState::LIGHT_GRAY_WOOL);
        }
    }

    commands.spawn(layer);

    commands.spawn(PlayerListEntryBundle {
        uuid: UniqueId(PLAYER_UUID_1),
        display_name: DisplayName(Some("persistent entry with no ping".into())),
        ..Default::default()
    });
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
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        *game_mode = GameMode::Creative;

        client.send_chat_message(
            "Please open your player list (tab key)."
                .italic()
                .color(Color::WHITE),
        );
    }
}

fn override_display_name(mut clients: Query<&mut DisplayName, (Added<DisplayName>, With<Client>)>) {
    for mut display_name in &mut clients {
        display_name.0 = Some("ඞ".color(Color::rgb(255, 87, 66)));
    }
}

fn update_player_list(
    mut player_list: ResMut<PlayerList>,
    server: Res<Server>,
    mut entries: Query<(Entity, &UniqueId, &mut DisplayName), With<PlayerListEntry>>,
    mut commands: Commands,
) {
    let tick = server.current_tick();

    player_list.set_header("Current tick: ".into_text() + tick);
    player_list
        .set_footer("Current tick but in purple: ".into_text() + tick.color(Color::LIGHT_PURPLE));

    if tick % 5 == 0 {
        for (_, uuid, mut display_name) in &mut entries {
            if uuid.0 == PLAYER_UUID_1 {
                let mut rng = rand::thread_rng();
                let color = Color::rgb(rng.gen(), rng.gen(), rng.gen());

                let new_name = display_name.0.clone().unwrap_or_default().color(color);
                display_name.0 = Some(new_name);
            }
        }
    }

    if tick % 20 == 0 {
        if let Some((entity, _, _)) = entries.iter().find(|(_, uuid, _)| uuid.0 == PLAYER_UUID_2) {
            commands.entity(entity).insert(Despawned);
        } else {
            commands.spawn(PlayerListEntryBundle {
                uuid: UniqueId(PLAYER_UUID_2),
                display_name: DisplayName(Some("Hello!".into())),
                ping: Ping(300),
                ..Default::default()
            });
        }
    }
}
