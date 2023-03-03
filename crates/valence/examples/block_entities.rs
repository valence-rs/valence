use valence::client::despawn_disconnected_clients;
use valence::client::event::{default_event_handler, ChatMessage, PlayerInteractBlock};
use valence::prelude::*;
use valence_nbt::{compound, List};
use valence_protocol::types::Hand;

const FLOOR_Y: i32 = 64;
const SIGN_POS: [i32; 3] = [3, FLOOR_Y + 1, 2];
const SKULL_POS: BlockPos = BlockPos::new(3, FLOOR_Y + 1, 3);

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_system_to_stage(EventLoop, event_handler)
        .add_system_set(PlayerList::default_system_set())
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .run();
}

fn setup(mut commands: Commands, server: Res<Server>) {
    let mut instance = server.new_instance(DimensionId::default());

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    for z in 0..16 {
        for x in 0..8 {
            instance.set_block([x, FLOOR_Y, z], BlockState::WHITE_CONCRETE);
        }
    }

    instance.set_block(
        [3, FLOOR_Y + 1, 1],
        BlockState::CHEST.set(PropName::Facing, PropValue::West),
    );
    instance.set_block(
        SIGN_POS,
        Block::with_nbt(
            BlockState::OAK_SIGN.set(PropName::Rotation, PropValue::_4),
            compound! {
                "Text1" => "Type in chat:".color(Color::RED),
            },
        ),
    );
    instance.set_block(
        SKULL_POS,
        BlockState::PLAYER_HEAD.set(PropName::Rotation, PropValue::_12),
    );

    commands.spawn(instance);
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

fn event_handler(
    clients: Query<&Client>,
    mut messages: EventReader<ChatMessage>,
    mut block_interacts: EventReader<PlayerInteractBlock>,
    mut instances: Query<&mut Instance>,
) {
    let mut instance = instances.single_mut();
    for ChatMessage {
        client, message, ..
    } in messages.iter()
    {
        let Ok(client) = clients.get(*client) else {
            continue
        };

        let mut sign = instance.block_mut(SIGN_POS).unwrap();
        let nbt = sign.nbt_mut().unwrap();
        nbt.insert("Text2", message.to_string().color(Color::DARK_GREEN));
        nbt.insert("Text3", format!("~{}", client.username()).italic());
    }

    for PlayerInteractBlock {
        client,
        position,
        hand,
        ..
    } in block_interacts.iter()
    {
        if *hand == Hand::Main && *position == SKULL_POS {
            let Ok(client) = clients.get(*client) else {
                continue
            };

            let Some(textures) = client.properties().iter().find(|prop| prop.name == "textures") else {
                continue
            };

            let mut skull = instance.block_mut(SKULL_POS).unwrap();
            let nbt = skull.nbt_mut().unwrap();
            *nbt = compound! {
                "SkullOwner" => compound! {
                    "Id" => client.uuid(),
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
