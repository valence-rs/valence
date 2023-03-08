use bevy_app::App;
use tracing::warn;
use valence::client::despawn_disconnected_clients;
use valence::client::event::{default_event_handler, ChatMessage, CommandExecution};
use valence::prelude::*;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()).with_connection_mode(ConnectionMode::Offline))
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
            instance.set_block([x, SPAWN_Y, z], BlockState::BEDROCK);
        }
    }

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<(&mut Client, &mut Position, &mut Location, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut client, pos, loc, game_mode) in &mut clients {
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        loc.0 = instances.single();
        *game_mode = GameMode::Adventure;
        client.send_message("Welcome to Valence! Talk about something.".italic());
    }
}

fn handle_message_events(
    mut clients: Query<(&mut Client, &Username)>,
    mut messages: EventReader<ChatMessage>,
) {
    for message in messages.iter() {
        let Ok(client) = clients.get_component::<Client>(message.client) else {
            warn!("Unable to find client for message: {:?}", message);
            continue;
        };

        let message = message.message.to_string();

        let formatted = format!("<{}>: ", username).bold().color(Color::YELLOW)
            + message.not_bold().color(Color::WHITE);

        // TODO: write message to instance buffer.
        for mut client in &mut clients {
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
