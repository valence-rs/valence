use valence::client::despawn_disconnected_clients;
use valence::client::event::{default_event_handler, CommandExecution};
use valence::prelude::*;

const SPAWN_Y: i32 = 64;

pub fn main() {
    tracing_subscriber::fmt().init();

    App::new()
        .add_plugin(ServerPlugin::new(()))
        .add_startup_system(setup)
        .add_system(init_clients)
        .add_systems((default_event_handler, interpret_command).in_schedule(EventLoopSchedule))
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
            instance.set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(instance);
}

fn init_clients(
    mut clients: Query<
        (
            &mut Client,
            &mut Position,
            &mut Location,
            &mut GameMode,
            &mut OpLevel,
        ),
        Added<Client>,
    >,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut client, mut pos, mut loc, mut game_mode, mut op_level) in &mut clients {
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        loc.0 = instances.single();
        *game_mode = GameMode::Creative;
        op_level.set(2); // required to use F3+F4, eg /gamemode
        client.send_message("Welcome to Valence! Use F3+F4 to change gamemode.".italic());
    }
}

fn interpret_command(
    mut clients: Query<(&mut Client, &OpLevel, &mut GameMode)>,
    mut events: EventReader<CommandExecution>,
) {
    for event in events.iter() {
        let Ok((mut client, op_level, mut game_mode)) = clients.get_mut(event.client) else {
            continue;
        };

        let mut args = event.command.split_whitespace();

        if args.next() == Some("gamemode") {
            if op_level.get() < 2 {
                // not enough permissions to use gamemode command
                continue;
            }

            let mode = match args.next().unwrap_or_default() {
                "adventure" => GameMode::Adventure,
                "creative" => GameMode::Creative,
                "survival" => GameMode::Survival,
                "spectator" => GameMode::Spectator,
                _ => {
                    client.send_message("Invalid gamemode.".italic());
                    continue;
                }
            };
            *game_mode = mode;
            client.send_message(format!("Set gamemode to {mode:?}.").italic());
        }
    }
}
