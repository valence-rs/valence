use valence::client::despawn_disconnected_clients;
use valence::client::event::{default_event_handler, ChatMessage, PlayerInteractBlock};
use valence::nbt::{compound, List};
use valence::prelude::*;
use valence::protocol::types::Hand;

const FLOOR_Y: i32 = 64;
const SIGN_POS: [i32; 3] = [3, FLOOR_Y + 1, 2];
const SKULL_POS: BlockPos = BlockPos::new(3, FLOOR_Y + 1, 3);

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_startup_system(setup)
        .add_systems((
            default_event_handler.in_schedule(EventLoopSchedule),
            event_handler.in_schedule(EventLoopSchedule),
            init_clients,
            despawn_disconnected_clients,
        ))
        .add_systems(PlayerList::default_systems())
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
    mut clients: Query<
        (
            Entity,
            &UniqueId,
            &mut Position,
            &mut Yaw,
            &mut Location,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    instances: Query<Entity, With<Instance>>,
    mut commands: Commands,
) {
    for (entity, uuid, mut pos, mut yaw, mut loc, mut game_mode) in &mut clients {
        pos.set([1.5, FLOOR_Y as f64 + 1.0, 1.5]);
        yaw.0 = -90.0;
        loc.0 = instances.single();
        *game_mode = GameMode::Creative;
        
        commands
            .entity(entity)
            .insert(McEntity::with_uuid(EntityKind::Player, loc.0, uuid.0));
    }
}

fn event_handler(
    clients: Query<(&Username, &Properties, &UniqueId)>,
    mut messages: EventReader<ChatMessage>,
    mut block_interacts: EventReader<PlayerInteractBlock>,
    mut instances: Query<&mut Instance>,
) {
    let mut instance = instances.single_mut();
    for ChatMessage {
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

    for PlayerInteractBlock {
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
