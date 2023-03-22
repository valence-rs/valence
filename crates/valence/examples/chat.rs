#![allow(clippy::type_complexity)]

use tracing::warn;
use valence::client::event::{ChatMessage, CommandExecution};
use valence::client::{default_event_handler, despawn_disconnected_clients};
use valence::entity::player::PlayerBundle;
use valence::prelude::*;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_systems(
            (
                default_event_handler,
                handle_message_events,
                handle_command_events,
            )
                .in_schedule(EventLoopSchedule),
        )
        .add_systems(PlayerList::default_systems())
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

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block([x, SPAWN_Y, z], BlockState::OAK_PLANKS);
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
        *game_mode = GameMode::Adventure;
        client.send_message("Welcome to Valence! Talk about something.".italic());

        commands.entity(entity).insert(PlayerBundle {
            location: Location(instances.single()),
            position: Position::new([0.0, SPAWN_Y as f64 + 1.0, 0.0]),
            uuid: *uuid,
            ..Default::default()
        });
    }
}

fn handle_message_events(
    mut clients: Query<(&mut Client, &Username)>,
    mut messages: EventReader<ChatMessage>,
) {
    for message in messages.iter() {
        let Ok(username) = clients.get_component::<Username>(message.client) else {
            warn!("Unable to find client for message: {:?}", message);
            continue;
        };

        let message = message.message.to_string();

        let formatted = format!("<{}>: ", username.0).bold().color(Color::YELLOW)
            + message.not_bold().color(Color::WHITE);

        // TODO: write message to instance buffer.
        for (mut client, _) in &mut clients {
            client.send_message(formatted.clone());
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
