#![allow(clippy::type_complexity)]

use valence::nbt::{compound, List};
use valence::prelude::*;
use valence_client::chat::ChatMessageEvent;
use valence_client::interact_block::InteractBlockEvent;

const FLOOR_Y: i32 = 64;
const SIGN_POS: [i32; 3] = [3, FLOOR_Y + 1, 2];
const SKULL_POS: BlockPos = BlockPos::new(3, FLOOR_Y + 1, 3);

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_systems((event_handler, init_clients, despawn_disconnected_clients))
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

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
    mut clients: Query<(&mut Location, &mut Position, &mut Look, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut loc, mut pos, mut look, mut game_mode) in &mut clients {
        loc.0 = instances.single();
        pos.set([1.5, FLOOR_Y as f64 + 1.0, 1.5]);
        *look = Look::new(-90.0, 0.0);

        *game_mode = GameMode::Creative;
    }
}

fn event_handler(
    clients: Query<(&Username, &Properties, &UniqueId)>,
    mut messages: EventReader<ChatMessageEvent>,
    mut block_interacts: EventReader<InteractBlockEvent>,
    mut instances: Query<&mut Instance>,
) {
    let mut instance = instances.single_mut();
    for ChatMessageEvent {
        client, message, ..
    } in messages.iter()
    {
        let Ok((username, _, _)) = clients.get(*client) else {
            continue
        };

        let mut sign = instance.block_mut(SIGN_POS).unwrap();
        let nbt = sign.nbt_mut().unwrap();
        nbt.insert("Text2", message.to_string().color(Color::DARK_GREEN));
        nbt.insert("Text3", format!("~{}", username).italic());
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
                continue
            };

            let Some(textures) = properties.textures() else {
                continue;
            };

            let mut skull = instance.block_mut(SKULL_POS).unwrap();
            let nbt = skull.nbt_mut().unwrap();
            *nbt = compound! {
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
