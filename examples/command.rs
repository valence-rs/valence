#![allow(clippy::type_complexity)]

use clap::arg;
use valence::prelude::*;
use valence::protocol::packets::play::command_tree_s2c::Parser;
use valence_command::command_graph::CommandGraphBuilder;
use valence_command::command_scopes::CommandScopes;
use valence_command::handler::{CommandExecutionEvent, CommandHandler};
use valence_command::{arg_parser, Command, CommandArgSet, CommandScopeRegistry};
use valence_command_derive::Command;

const SPAWN_Y: i32 = 64;

// pub enum TeleportResult {
//     ExecutorToLocation(arg_parser::Vec3),
//     ExecutorToTarget(String),
//     TargetToTarget((String, String)),
//     TargetToLocation((String, arg_parser::Vec3)),
// }

// #[derive(Command)]
// #[paths("selectfruit", "select fruit", "sf")]
// #[scopes("valence:command:teleport")]
// enum SelectFruit {
//     #[paths = "apple"]
//     // this path is from the perant: selectfruit so `/selectfruit apple` will be here
//     Apple,
//     #[paths = "banana"]
//     Banana,
//     #[paths = "Strawberry {breed} with_name {name} {kind?}"]
//     // this could be `/selectfruit banana green` or /selectfruit banana
//     // the macro should be able to detect the fact it is optional and register two executables;
//     // one has no args and the other has the optional arg
//     Strawberry {
//         breed: arg_parser::Vec3,
//         name: String,
//         kind: Option<f32>,
//     },
//     #[paths("orange", "o")]
//     Orange,
// }
#[derive(Command, Debug)]
#[paths("teleport", "tp")]
#[scopes("valence:command:teleport")]
enum Teleport {
    #[paths = "{location}"]
    ExecutorToLocation { location: arg_parser::Vec3 },
    #[paths = "{target}"]
    ExecutorToTarget { target: String },
    #[paths = "{from} {to}"]
    TargetToTarget { from: String, to: String },
    #[paths = "{target} {location}"]
    TargetToLocation {
        target: String,
        location: arg_parser::Vec3,
    },
}


// #[derive(Suggestions)] // I'd want this to assume snake case unless manully set
// enum Strawberry {
//     Red,
//     Green
// }

// #[derive(Resource, Clone)]
// struct TeleportCommand;
//
// impl Command for TeleportCommand {
//     type CommandExecutables = TeleportResult;
//
//     fn assemble_graph(&self, command_graph: &mut CommandGraphBuilder<Self::CommandExecutables>) {
//         let teleport = command_graph
//             .root()
//             .literal("teleport")
//             .with_scopes(vec!["valence:command:teleport"])
//             .id();
//
//         // tp alias
//         command_graph
//             .root()
//             .literal("tp")
//             .with_scopes(vec!["valence:command:teleport"])
//             .redirect_to(teleport);
//
//         // executor to vec3 target
//         command_graph
//             .at(teleport)
//             .argument("destination:location")
//             .with_parser(Parser::Vec3)
//             .with_executable(|s| {
//                 TeleportResult::ExecutorToLocation(arg_parser::Vec3::parse_args(s).unwrap())
//             });
//
//         // executor to entity target
//         command_graph
//             .at(teleport)
//             .argument("destination:entity")
//             .with_parser(Parser::Entity {
//                 only_players: false,
//                 single: true,
//             })
//             .with_executable(|s| {
//                 TeleportResult::ExecutorToTarget(arg_parser::EntitySelector::parse_args(s).unwrap())
//             });
//
//         let targeted_teleport = command_graph
//             .root()
//             .at(teleport)
//             .argument("target:entity")
//             .with_parser(Parser::Entity {
//                 only_players: false,
//                 single: false,
//             })
//             .id();
//
//         // target to target
//         command_graph
//             .at(targeted_teleport)
//             .argument("destination:entity")
//             .with_parser(Parser::Entity {
//                 only_players: false,
//                 single: true,
//             })
//             .with_executable(|s| {
//                 TeleportResult::TargetToTarget(
//                     <(arg_parser::EntitySelector, arg_parser::EntitySelector)>::parse_args(s)
//                         .unwrap(),
//                 )
//             });
//         // target to location
//         command_graph
//             .at(targeted_teleport)
//             .argument("destination:location")
//             .with_parser(Parser::Vec3)
//             .with_executable(|s| {
//                 TeleportResult::TargetToLocation(
//                     <(arg_parser::EntitySelector, arg_parser::Vec3)>::parse_args(s).unwrap(),
//                 )
//             });
//     }
// }

pub fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            CommandHandler::from_command(TeleportCommand),
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                init_clients,
                despawn_disconnected_clients,
                toggle_perms_on_sneak,
                handle_teleport_command,
            ),
        )
        .run();
}

fn handle_teleport_command(
    mut events: EventReader<CommandExecutionEvent<TeleportCommand>>,
    mut clients: Query<(&mut Client, &mut Position)>,
    usernames: Query<(Entity, &Username)>, // mut commands: Commands
) {
    for event in events.iter() {
        match &event.result {
            Teleport::ExecutorToLocation { location } => {
                let (client, pos) = &mut clients.get_mut(event.executor).unwrap();
                pos.0.x = location.x.get(pos.0.x as f32) as f64;
                pos.0.y = location.y.get(pos.0.y as f32) as f64;
                pos.0.z = location.z.get(pos.0.z as f32) as f64;

                client.send_chat_message(format!(
                    "Teleport command executor -> location executed with data:\n {:#?}",
                    &event.result
                ));
            }
            Teleport::ExecutorToTarget { target: raw_target } => {
                let target = usernames.iter().find(|(_, name)| name.0 == *raw_target);

                match target {
                    None => {
                        let client = &mut clients.get_mut(event.executor).unwrap().0;
                        client.send_chat_message(format!("Could not find target: {}", raw_target));
                    }
                    Some(target_entity) => {
                        let target_pos = clients.get(target_entity.0).unwrap().1 .0;
                        let pos = &mut clients.get_mut(event.executor).unwrap().1 .0;
                        pos.x = target_pos.x;
                        pos.y = target_pos.y;
                        pos.z = target_pos.z;
                    }
                }

                let client = &mut clients.get_mut(event.executor).unwrap().0;
                client.send_chat_message(format!(
                    "Teleport command executor -> target executed with data:\n {:#?}",
                    &event.result
                ));
            }
            Teleport::TargetToTarget { from, to } => {
                let from_target = usernames.iter().find(|(_, name)| name.0 == *from);
                let to_target = usernames.iter().find(|(_, name)| name.0 == *to);

                let client = &mut clients.get_mut(event.executor).unwrap().0;
                client.send_chat_message(format!(
                    "Teleport command target -> location with data:\n {:#?}",
                    &event.result
                ));
                match from_target {
                    None => {
                        client.send_chat_message(format!("Could not find target: {}", from));
                    }
                    Some(from_target_entity) => match to_target {
                        None => {
                            client.send_chat_message(format!("Could not find target: {}", to));
                        }
                        Some(to_target_entity) => {
                            let target_pos = *clients.get(to_target_entity.0).unwrap().1;
                            let (from_client, from_pos) =
                                &mut clients.get_mut(from_target_entity.0).unwrap();
                            from_pos.0 = target_pos.0;

                            from_client.send_chat_message(format!(
                                "You have been teleported to {}",
                                to_target_entity.1
                            ));

                            let to_client = &mut clients.get_mut(to_target_entity.0).unwrap().0;
                            to_client.send_chat_message(format!(
                                "{} has been teleported to your location",
                                from_target_entity.1
                            ));
                        }
                    },
                }
            }
            Teleport::TargetToLocation {
                target: target_raw,
                location,
            } => {
                let target = usernames.iter().find(|(_, name)| name.0 == *target_raw);

                let client = &mut clients.get_mut(event.executor).unwrap().0;
                client.send_chat_message(format!(
                    "Teleport command target -> location with data:\n {:#?}",
                    &event.result
                ));
                match target {
                    None => {
                        client.send_chat_message(format!("Could not find target: {}", target_raw));
                    }
                    Some(target_entity) => {
                        let (client, pos) = &mut clients.get_mut(target_entity.0).unwrap();
                        pos.0.x = location.x.get(pos.0.x as f32) as f64;
                        pos.0.y = location.y.get(pos.0.y as f32) as f64;
                        pos.0.z = location.z.get(pos.0.z as f32) as f64;

                        client.send_chat_message(format!(
                            "Teleport command executor -> location executed with data:\n {:#?}",
                            &event.result
                        ));
                    }
                }
            }
        }
    }
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    mut permissions: ResMut<CommandScopeRegistry>,
) {
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            layer
                .chunk
                .set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    permissions.add_scope("valence:command:teleport");

    commands.spawn(layer);
}

fn init_clients(
    mut clients: Query<
        (
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut CommandScopes,
            &mut Position,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut permissions,
        mut pos,
        mut game_mode,
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);

        pos.0 = [0.0, SPAWN_Y as f64 + 1.0, 0.0].into();
        *game_mode = GameMode::Creative;

        permissions.add("valence:command:teleport");
    }
}

fn toggle_perms_on_sneak(
    mut clients: Query<&mut CommandScopes>,
    mut events: EventReader<SneakEvent>,
) {
    for event in events.iter() {
        let Ok(mut perms) = clients.get_mut(event.client) else {
            continue;
        };
        if event.state == SneakState::Start {
            match perms.scopes.len() {
                0 => perms.add("valence:command:teleport"),
                1 => perms.remove("valence:command:teleport"),
                _ => panic!("Too many permissions!"),
            };
        }
    }
}
