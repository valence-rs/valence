use valence_new::client::despawn_disconnected_clients;
use valence_new::client::event::{default_event_handler, ChatCommand};
use valence_new::prelude::*;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_system_to_stage(EventLoop, default_event_handler)
        .add_system_to_stage(EventLoop, interpret_command)
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

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block_state([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    world.spawn(instance);
}

fn init_clients(
    mut clients: Query<&mut Client, Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    let instance = instances.get_single().unwrap();

    for mut client in &mut clients {
        client.set_position([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        client.set_instance(instance);
        client.set_game_mode(GameMode::Creative);
        client.set_op_level(4); // required to use F3+F4, eg /gamemode
        client.send_message("Welcome to Valence! Use F3+F4 to change gamemode.".italic());
    }
}

fn interpret_command(mut clients: Query<&mut Client>, mut events: EventReader<ChatCommand>) {
    for event in events.iter() {
        let Ok(mut client) = clients.get_component_mut::<Client>(event.client) else {
            continue;
        };

        let mut args = event.command.split_whitespace();
        let command = args.next().unwrap_or_default();

        match command {
            "gamemode" => {
                if client.op_level() < 2 {
                    // not enough permissions to use gamemode command
                    continue;
                }

                let mode = args.next().unwrap_or_default();
                let mode = match mode {
                    "adventure" => GameMode::Adventure,
                    "creative" => GameMode::Creative,
                    "survival" => GameMode::Survival,
                    "spectator" => GameMode::Spectator,
                    _ => {
                        client.send_message("Invalid gamemode.".italic());
                        continue;
                    }
                };
                client.set_game_mode(mode);
                client.send_message(format!("Set gamemode to {:?}.", mode).italic());
            }
            _ => { /* ignored */ }
        }
    }
}
