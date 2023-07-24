#![allow(clippy::type_complexity)]

use std::borrow::Cow;

use valence::prelude::*;
use valence_command::command::{CommandArguments, RealCommandExecutor};
use valence_command::entity::NodeEntityCommandGet;

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
            instance.insert_chunk([x, z], Chunk::default());
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
                child.name(Cow::Borrowed("set_spectator")).parser::<bool>(()).executor(
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
