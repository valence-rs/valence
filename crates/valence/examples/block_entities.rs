use valence::client::despawn_disconnected_clients;
use valence::client::event::{default_event_handler, ChatMessage, UseItemOnBlock};
use valence::prelude::*;
use valence_nbt::{compound, List};
use valence_protocol::block::{BlockEntity, BlockEntityKind, PropName, PropValue};

const FLOOR_Y: i32 = 64;
const SIGN_POS: [i32; 3] = [3, FLOOR_Y + 1, 2];
const SKULL_POS: BlockPos = BlockPos::new(3, FLOOR_Y + 1, 3);

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_system_to_stage(EventLoop, chat_handler)
        .add_system_set(PlayerList::default_system_set())
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .run();
}

fn setup(world: &mut World) {
    let mut instance = world
        .resource::<Server>()
        .new_instance(DimensionId::default());

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    for z in 0..16 {
        for x in 0..8 {
            instance.set_block_state([x, FLOOR_Y, z], BlockState::WHITE_CONCRETE);
        }
    }

    instance.set_block_state(
        [3, FLOOR_Y + 1, 1],
        BlockState::CHEST.set(PropName::Facing, PropValue::West),
    );

    instance.set_block_state(
        SIGN_POS,
        BlockState::OAK_SIGN.set(PropName::Rotation, PropValue::_4),
    );

    instance.set_block_entity(
        SIGN_POS,
        BlockEntity {
            kind: BlockEntityKind::Sign,
            nbt: compound! {
                "Text1" => r#"{"text": "Type in chat:", "color": "red"}"#
            },
        },
    );

    instance.set_block_state(
        SKULL_POS,
        BlockState::PLAYER_HEAD.set(PropName::Rotation, PropValue::_12),
    );

    world.spawn(instance);
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for mut client in &mut clients {
        client.set_position([1.5, FLOOR_Y as f64 + 1.0, 1.5]);
        client.set_yaw(-90.0);
        client.set_instance(instances.single());
        client.set_game_mode(GameMode::Creative);
    }
}

fn chat_handler(
    clients: Query<&Client>,
    mut messages: EventReader<ChatMessage>,
    mut block_interacts: EventReader<UseItemOnBlock>,
    mut instances: Query<&mut Instance>,
) {
    let mut instance = instances.single_mut();
    for ChatMessage {
        client, message, ..
    } in messages.iter()
    {
        let text = message.to_string().color(Color::DARK_GREEN);
        let text = serde_json::to_string(&text).unwrap();

        let Ok(client) = clients.get(*client) else {
            continue
        };
        let author = format!("~{}", client.username()).italic();
        let author = serde_json::to_string(&author).unwrap();

        let Some(mut sign) = instance.block_entity(SIGN_POS) else {
            continue
        };

        sign.nbt.insert("Text2", text);
        sign.nbt.insert("Text3", author);

        instance.set_block_entity(SIGN_POS, sign);
    }

    for UseItemOnBlock {
        client, position, ..
    } in block_interacts.iter()
    {
        if *position == SKULL_POS {
            let Ok(client) = clients.get(*client) else {
                continue
            };

            let uuid: [i32; 4] = unsafe { std::mem::transmute(client.uuid().as_u128()) };
            let mut uuid: Vec<_> = uuid.into();
            uuid.reverse();

            let Some(textures) = client.properties().iter().find(|prop| prop.name == "textures") else {
                continue
            };

            instance.set_block_entity(
                SKULL_POS,
                BlockEntity {
                    kind: BlockEntityKind::Skull,
                    nbt: compound! {
                        "SkullOwner" => compound! {
                            "Id" => uuid,
                            "Properties" => compound! {
                                "textures" => List::Compound(vec![compound! {
                                    "Value" => textures.value.clone(),
                                }])
                            }
                        }
                    },
                },
            );
        }
    }
}
