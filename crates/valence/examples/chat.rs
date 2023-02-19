use bevy_app::App;
use tracing::warn;
use valence::client::despawn_disconnected_clients;
use valence::client::event::{default_event_handler, ChatCommand, ChatMessage};
use valence::prelude::*;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_startup_system(setup)
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_system_to_stage(EventLoop, handle_message_events)
        .add_system_to_stage(EventLoop, handle_command_events)
        .add_system(init_clients)
        .add_system(despawn_disconnected_clients)
        .add_system_set(PlayerList::default_system_set())
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

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block([x, SPAWN_Y, z], BlockState::BEDROCK);
        }
    }

    world.spawn(instance);
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for mut client in &mut clients {
        client.set_position([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        client.set_instance(instances.single());
        client.set_game_mode(GameMode::Adventure);
        client.send_message("Welcome to Valence! Talk about something.".italic());
    }
}

fn handle_message_events(mut clients: Query<&mut Client>, mut messages: EventReader<ChatMessage>) {
    for message in messages.iter() {
        let Ok(client) = clients.get_component::<Client>(message.client) else {
            warn!("Unable to find client for message: {:?}", message);
            continue;
        };

        let message = message.message.to_string();

        let formatted = format!("<{}>: ", client.username())
            .bold()
            .color(Color::YELLOW)
            + message.into_text().not_bold().color(Color::WHITE);

        clients.par_for_each_mut(16, |mut client| {
            client.send_message(formatted.clone());
        })
    }
}

fn handle_command_events(mut clients: Query<&mut Client>, mut commands: EventReader<ChatCommand>) {
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
