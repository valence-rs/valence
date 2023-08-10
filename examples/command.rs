#![allow(clippy::type_complexity)]

use std::borrow::Cow;

use command::builder::{NodeCommands, NodeGraphCommands};
use command::command::{CommandExecutorBase, CommandExecutorBridge};
use command::nodes::NodeGraphInWorld;
use valence::prelude::*;
use valence_command::command::{CommandArguments, RealCommandExecutor};

const SPAWN_Y: i32 = 64;

pub fn main() {
    App::new()
        .insert_resource(NetworkSettings {
            connection_mode: ConnectionMode::Offline,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (init_clients, despawn_disconnected_clients))
        .run();
}

macro_rules! gamemode_node {
    ($gamemode:expr) => {
        |node| {
            node.execute(
                |In(command_arguments): In<CommandArguments>,
                 mut gamemode_query: Query<&mut GameMode>| {
                    if let RealCommandExecutor::Player(entity) = command_arguments.1 {
                        if let Ok(mut gamemode_player) = gamemode_query.get_mut(entity) {
                            gamemode_player.set_if_neq($gamemode);
                        }
                    }
                },
            );
        }
    };
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    graph: ResMut<NodeGraphInWorld>,
) {
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(instance);

    let mut commands = NodeGraphCommands { commands, graph };

    commands
        .spawn_literal_node("gamemode".into())
        .with_literal_child("survival".into(), gamemode_node!(GameMode::Survival))
        .with_literal_child("creative".into(), gamemode_node!(GameMode::Creative))
        .with_literal_child("spectator".into(), gamemode_node!(GameMode::Spectator))
        .with_literal_child("adventure".into(), gamemode_node!(GameMode::Adventure))
        .root_node_child();

    fn teleport_execute(
        In(mut arguments): In<CommandArguments>,
        mut query: Query<&mut Position>,
        mut cebridge: CommandExecutorBridge,
    ) {
        if let RealCommandExecutor::Player(client) = arguments.1 {
            let x = arguments.0.read::<i32>();
            let y = arguments.0.read::<i32>();
            let z = arguments.0.read::<i32>();
            if let Ok(mut position) = query.get_mut(client) {
                position.0 = DVec3::new(*x as _, *y as _, *z as _);
                cebridge.send_message(
                    arguments.1,
                    Text::text(format!("We teleported you to ({x} {y} {z})")),
                );
            }
        }
    }

    let teleport_id = commands
        .spawn_literal_node("teleport".into())
        .with_argument_child::<i32>("x".into(), Default::default(), |child| {
            child.with_argument_child::<i32>("y".into(), Default::default(), |child| {
                child.with_argument_child::<i32>("z".into(), Default::default(), |child| {
                    child.execute(teleport_execute);
                });
            });
        })
        .root_node_child()
        .id;

    commands
        .spawn_literal_node("tp".into())
        .set_redirect(teleport_id)
        .root_node_child();
}

fn init_clients(
    mut clients: Query<(&mut Location, &mut Position, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut loc, mut pos, mut game_mode) in &mut clients {
        loc.0 = instances.single();
        pos.set([0.5, SPAWN_Y as f64 + 1.0, 0.5]);
        *game_mode = GameMode::Creative;
    }
}
