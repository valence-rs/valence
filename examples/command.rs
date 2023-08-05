#![allow(clippy::type_complexity)]

use std::borrow::Cow;

use valence::prelude::*;
use valence_command::command::{CommandArguments, CommandExecutorBridge, RealCommandExecutor};
use valence_command::entity::NodeEntityCommandGet;
use valence_command::nodes::NodeSuggestion;

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

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
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

    commands
        .spawn_root_node(true)
        .with_child(|child| {
            child.name(Cow::Borrowed("creative")).executor(
                |In(arguments): In<CommandArguments>, mut query: Query<&mut GameMode>| {
                    if let RealCommandExecutor::Player(client) = arguments.1 {
                        if let Ok(mut game_mode) = query.get_mut(client) {
                            *game_mode = GameMode::Creative;
                        }
                    }
                },
            );
        })
        .with_child(|child| {
            child.name(Cow::Borrowed("survival")).executor(
                |In(arguments): In<CommandArguments>, mut query: Query<&mut GameMode>| {
                    if let RealCommandExecutor::Player(client) = arguments.1 {
                        if let Ok(mut game_mode) = query.get_mut(client) {
                            *game_mode = GameMode::Survival;
                        }
                    }
                },
            );
        })
        .with_child(|child| {
            child.name(Cow::Borrowed("spectator")).with_child(|child| {
                child.name(Cow::Borrowed("set_spectator")).parser::<bool>(()).suggestions(Some(NodeSuggestion::AskServer)).executor(
                    |In(mut arguments): In<CommandArguments>, mut query: Query<&mut GameMode>| {
                        let enable = arguments.0.read::<bool>();
                        if *enable {
                            if let RealCommandExecutor::Player(client) = arguments.1 {
                                if let Ok(mut game_mode) = query.get_mut(client) {
                                    *game_mode = GameMode::Spectator;
                                }
                            }
                        }
                    },
                );
            });
        })
        .with_child(|child| {
            child.name(Cow::Borrowed("tp")).with_child(|child| {
                child.name(Cow::Borrowed("tp.x")).parser::<i32>(Default::default()).with_child(|child| {
                    child.name(Cow::Borrowed("tp.y")).parser::<i32>(Default::default())
                        .with_child(|child| {
                            child.name(Cow::Borrowed("tp.z")).parser::<i32>(Default::default())
                                .executor(|In(mut arguments): In<CommandArguments>, mut query: Query<&mut Position>, mut cebridge: CommandExecutorBridge| {
                                    // CommandExecutorBridge is a SystemParam
                                    if let RealCommandExecutor::Player(client) = arguments.1 {
                                        let x = arguments.0.read::<i32>();
                                        let y = arguments.0.read::<i32>();
                                        let z = arguments.0.read::<i32>();
                                        if let Ok(mut position) = query.get_mut(client) {
                                            position.0 = DVec3::new(*x as _, *y as _, *z as _);
                                            cebridge.send_message(arguments.1, Text::text(format!("We teleported you to ({x} {y} {z})")));
                                        }
                                    }
                                });
                        })
                        .with_child(|child| {
                            child.name(Cow::Borrowed("tp.zfloat")).parser::<f32>(Default::default())
                                .executor(|In(mut arguments): In<CommandArguments>, mut query: Query<&mut Position>, mut cebridge: CommandExecutorBridge| {
                                    // CommandExecutorBridge is a SystemParam
                                    if let RealCommandExecutor::Player(client) = arguments.1 {
                                        let x = arguments.0.read::<i32>();
                                        let y = arguments.0.read::<i32>();
                                        let z = arguments.0.read::<f32>();
                                        if let Ok(mut position) = query.get_mut(client) {
                                            position.0 = DVec3::new(*x as _, *y as _, *z as _);
                                            cebridge.send_message(arguments.1, Text::text(format!("We teleported you to ({x} {y} {z} (f32))")));
                                        }
                                    }
                                });
                        });
                });
            });
        });
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
