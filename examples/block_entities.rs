#![allow(clippy::type_complexity)]

use valence::interact_block::InteractBlockEvent;
use valence::chat::message::ChatMessageEvent;
use valence::nbt::{compound, List};
use valence::prelude::*;

const FLOOR_Y: i32 = 64;
const SIGN_POS: [i32; 3] = [3, FLOOR_Y + 1, 2];
const SKULL_POS: BlockPos = BlockPos::new(3, FLOOR_Y + 1, 3);

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (event_handler, init_clients, despawn_disconnected_clients),
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

    for z in 0..16 {
        for x in 0..8 {
            layer
                .chunk
                .set_block([x, FLOOR_Y, z], BlockState::WHITE_CONCRETE);
        }
    }

    layer.chunk.set_block(
        [3, FLOOR_Y + 1, 1],
        BlockState::CHEST.set(PropName::Facing, PropValue::West),
    );

    layer.chunk.set_block(
        SIGN_POS,
        Block {
            state: BlockState::OAK_SIGN.set(PropName::Rotation, PropValue::_4),
            nbt: Some(compound! {
                "front_text" => compound! {
                    "messages" => List::String(vec![
                        // All 4 lines are required, otherwise no text is displayed.
                        "Type in chat:".color(Color::RED).into(),
                        "".into_text().into(),
                        "".into_text().into(),
                        "".into_text().into(),
                    ]),
                }
            }),
        },
    );

    layer.chunk.set_block(
        SKULL_POS,
        BlockState::PLAYER_HEAD.set(PropName::Rotation, PropValue::_12),
    );

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
        pos.set([0.0, FLOOR_Y as f64 + 1.0, 0.0]);
        *game_mode = GameMode::Creative;
    }
}

fn event_handler(
    clients: Query<(&Username, &Properties, &UniqueId)>,
    mut messages: EventReader<ChatMessageEvent>,
    mut block_interacts: EventReader<InteractBlockEvent>,
    mut layers: Query<&mut ChunkLayer>,
) {
    let mut layer = layers.single_mut();

    for ChatMessageEvent {
        client, message, ..
    } in messages.iter()
    {
        let Ok((username, _, _)) = clients.get(*client) else {
            continue;
        };

        let nbt = layer.block_entity_mut(SIGN_POS).unwrap();
        nbt.merge(compound! {
            "front_text" => compound! {
                "messages" => List::String(vec![
                    "Type in chat:".color(Color::RED).into(),
                    message.to_string().color(Color::DARK_GREEN).into(),
                    format!("~{username}").italic().into(),
                    "".into_text().into(),
                ]),
            },
        });
    }

    for InteractBlockEvent {
        client,
        position,
        hand,
        ..
    } in block_interacts.iter()
    {
        if *hand == Hand::Main && *position == SKULL_POS {
            let Ok((_, properties, uuid)) = clients.get(*client) else {
                continue;
            };

            let Some(textures) = properties.textures() else {
                continue;
            };

            *layer.block_entity_mut(SKULL_POS).unwrap() = compound! {
                "SkullOwner" => compound! {
                    "Id" => uuid.0,
                    "Properties" => compound! {
                        "textures" => List::Compound(vec![compound! {
                            "Value" => textures.value.clone(),
                        }])
                    }
                }
            };
        }
    }
}
