#![allow(clippy::type_complexity)]

use tracing::warn;
use valence::chat::ChatState;
use valence::client::chat::{ChatMessage, CommandExecution};
use valence::client::despawn_disconnected_clients;
use valence::prelude::*;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_systems((
            init_clients,
            despawn_disconnected_clients,
            handle_message_events.in_schedule(EventLoopSchedule),
            handle_command_events.in_schedule(EventLoopSchedule),
        ))
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
    mut clients: Query<(&mut Client, &mut Location, &mut Position, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut client, mut loc, mut pos, mut game_mode) in &mut clients {
        *game_mode = GameMode::Creative;
        loc.0 = instances.single();
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);

        client.send_message("Welcome to Valence! Say something.".italic());
    }
}

fn handle_message_events(
    mut clients: Query<(&mut Client, &mut ChatState)>,
    names: Query<&Username>,
    mut messages: EventReader<ChatMessage>,
) {
    for message in messages.iter() {
        let sender_name = names.get(message.client).expect("Error getting username");
        // Need to find better way. Username is sender, while client and chat state are
        // recievers. Maybe try to add a chat feature to Client.
        for (mut client, mut state) in clients.iter_mut() {
            state
                .as_mut()
                .send_chat_message(client.as_mut(), sender_name, message)
                .expect("Error sending message");
        }
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
