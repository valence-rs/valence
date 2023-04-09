#![allow(clippy::type_complexity)]

use tracing::{warn, Level};
use valence::client::despawn_disconnected_clients;
// TODO: Add CommandExecution event
use valence::client::misc::CommandExecution;
use valence::entity::player::PlayerEntityBundle;
use valence::prelude::*;
use valence::secure_chat::SecureChatPlugin;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_plugin(SecureChatPlugin)
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system(handle_command_events.in_schedule(EventLoopSchedule))
        .add_systems(PlayerList::default_systems())
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Query<&DimensionType>,
    biomes: Query<&Biome>,
) {
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<(Entity, &UniqueId, &mut Client, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
    mut commands: Commands,
) {
    for (entity, uuid, mut client, mut game_mode) in &mut clients {
        *game_mode = GameMode::Creative;
        client.send_message("Welcome to Valence! Say something.".italic());

        commands.entity(entity).insert(PlayerEntityBundle {
            location: Location(instances.single()),
            position: Position::new([0.0, SPAWN_Y as f64 + 1.0, 0.0]),
            uuid: *uuid,
            ..Default::default()
        });
    }
}

fn handle_command_events(
    mut clients: Query<&mut Client>,
    mut commands: EventReader<CommandExecution>,
) {
    for command in commands.iter() {
        let Ok(mut client) = clients.get_component_mut::<Client>(command.client) else {
            warn!("Unable to find client for message: {:?}", command);
            continue;
        };

        let message = command.command.to_string();

        let formatted =
            "You sent the command ".into_text() + ("/".into_text() + (message).into_text()).bold();

        client.send_message(formatted);
    }
}
