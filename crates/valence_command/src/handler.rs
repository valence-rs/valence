use std::collections::HashMap;
use std::fmt::{Debug};
use std::marker::PhantomData;

use bevy_app::{App, Plugin, PostStartup, Update};
use bevy_ecs::change_detection::{Res, ResMut};
use bevy_ecs::event::{Event, EventReader, EventWriter};
use bevy_ecs::prelude::{Entity, Resource};
use bevy_ecs::system::Query;
use petgraph::algo::all_simple_paths;

use petgraph::prelude::NodeIndex;
use petgraph::Graph;
use valence_server::client::Client;
use valence_server::event_loop::PacketEvent;
use valence_server::message::SendMessage;
use valence_server::protocol::packets::play::CommandExecutionC2s;

use crate::arg_parser::{ParseInput};
use crate::command_graph::{
    CommandEdgeType, CommandGraphBuilder, CommandNode, NodeData,
};
use crate::command_scopes::CommandScopes;
use crate::{Command, CommandRegistry, CommandScopeRegistry};

pub struct CommandHandler<T>
where
    T: Command,
{
    command: PhantomData<T>,
}

impl<T> CommandHandler<T>
where
    T: Command,
{
    pub fn from_command() -> Self {
        CommandHandler {
            command: PhantomData,
        }
    }
}

#[derive(Resource)]
struct CommandResource<T: Command + Send + Sync> {
    command: PhantomData<T>,
    executables: HashMap<NodeIndex, fn(&mut ParseInput) -> T>,
    parsers: HashMap<NodeIndex, fn(&mut ParseInput) -> bool>,
}

impl<T: Command + Send + Sync> CommandResource<T> {
    pub fn new() -> Self {
        CommandResource {
            command: PhantomData,
            executables: HashMap::new(),
            parsers: HashMap::new(),
        }
    }
}

#[derive(Event)]
pub struct CommandExecutionEvent<T>
where
    T: Command,
    T: Send + Sync + 'static,
{
    pub result: T,
    pub executor: Entity,
}

impl<T> Plugin for CommandHandler<T>
where
    T: Command + Send + Sync + Debug + 'static,
{
    fn build(&self, app: &mut App) {
        // println!("Registering command: {}", Self::command_name());

        app.add_event::<CommandExecutionEvent<T>>()
            .insert_resource(CommandResource::<T>::new())
            .add_systems(Update, command_event_system::<T>)
            .add_systems(PostStartup, command_startup_system::<T>);

        // println!("Registered command: {}", Self::command_name());
    }
}

fn command_startup_system<T>(
    mut registry: ResMut<CommandRegistry>,
    mut command: ResMut<CommandResource<T>>,
) where
    T: Command + Send + Sync + 'static,
{
    let mut executables = HashMap::new();
    let mut parsers = HashMap::new();
    let graph_builder =
        &mut CommandGraphBuilder::new(&mut registry, &mut executables, &mut parsers);
    T::assemble_graph(graph_builder);

    println!("Command graph: {}", &registry.graph);
    command.executables.extend(executables);
    command.parsers.extend(parsers);
}

/// this system reads incoming command events and prints them to the console
fn command_event_system<T>(
    mut packets: EventReader<PacketEvent>,
    registry: Res<CommandRegistry>,
    mut events: EventWriter<CommandExecutionEvent<T>>,
    command: ResMut<CommandResource<T>>,
    scope_registry: Res<CommandScopeRegistry>,
    scopes: Query<&CommandScopes>,
    mut clients: Query<&mut Client>,
) where
    T: Command + Send + Sync + Debug,
{
    for packet in packets.iter() {
        let client = packet.client;
        if let Some(packet) = packet.decode::<CommandExecutionC2s>() {
            println!("Received command: {:?}", packet);
            let executable_leafs = command.executables.keys().collect::<Vec<&NodeIndex>>();
            println!("Executable leafs: {:?}", executable_leafs);
            let root = registry.graph.root;
            for leaf in executable_leafs {
                // we want to find all possible paths from root to leaf then check if the path
                // matches the command
                let paths = all_simple_paths::<
                    Vec<NodeIndex>,
                    &Graph<CommandNode, CommandEdgeType>,
                >(&registry.graph.graph, root, *leaf, 0, None);

                let command_input = packet
                    .command
                    // .split_whitespace()
                    // .map(|s| s.to_string())
                    // .collect::<Vec<String>>()
                    ;
                let mut command_args = Vec::new();
                'paths: for path in paths {
                    let mut input = ParseInput::new(command_input);
                    for (i, node) in path.iter().enumerate() {
                        if i != 0 {
                            // the first node in path cannot be product of a redirect
                            // if this node is the product of a redirect, we want to skip it
                            // without this check the tp command would have to be written as
                            // /tp teleport <params> instead of /tp <params>
                            if registry
                                .graph
                                .graph
                                .edges_connecting(path[i - 1], path[i])
                                .clone()
                                .any(|edge| *edge.weight() == CommandEdgeType::Redirect)
                            {
                                continue;
                            }
                        }

                        let node_scopes = registry.graph.graph[path[i]].scopes.clone();
                        if !node_scopes.is_empty() {
                            let mut has_scope = false;
                            for scope in node_scopes {
                                if scope_registry
                                    .any_grants(scopes.get(client).unwrap().scopes.clone(), scope)
                                {
                                    has_scope = true;
                                    break;
                                }
                            }
                            if !has_scope {
                                break;
                            }
                        }

                        input.skip_whitespace();
                        match &registry.graph.graph[*node].data {
                            NodeData::Root => {}
                            NodeData::Literal { name } => match input.match_next(name) {
                                true => {}
                                false => continue 'paths,
                            },
                            NodeData::Argument { .. } => {
                                let parser = command.parsers.get(node).unwrap();
                                let before_cursor = input.cursor;
                                let valid = parser(&mut input);
                                let after_cursor = input.cursor;
                                if valid {
                                    command_args.push(
                                        input.input[before_cursor..after_cursor].to_string(),
                                    );
                                } else {
                                    continue 'paths;
                                }
                            }
                        }
                    }
                    if input.cursor == input.input.len() {
                        let mut command_args = ParseInput::new(command_args.join(" "));
                        println!("Executing command with args: {:?}", command_args);
                        let result = command.executables.get(&path[path.len() - 1]).unwrap()(
                            &mut command_args,
                        );
                        clients.get_mut(client).unwrap().send_chat_message(format!(
                            "executing command with info {:#?}",
                            result
                        ));
                        events.send(CommandExecutionEvent {
                            result,
                            executor: client,
                        });

                        break 'paths;
                    }

                        //
                        // match &registry.graph.graph[*node].data {
                        //     NodeData::Root => {}
                        //     NodeData::Literal { .. } => command_len += 1,
                        //     NodeData::Argument { parser, .. } => match parser_len(parser) {
                        //         ArgLen::Infinite => {
                        //             potentially_infinite = true;
                        //             command_len = 0;
                        //         }
                        //         ArgLen::Exact(num) => command_len += num,
                        //         ArgLen::Within(_) => {
                        //             potentially_infinite = true;
                        //             command_len = 0;
                        //         }
                        //         ArgLen::WithinExplicit(..) => {
                        //             potentially_infinite = true;
                        //             command_len = 0;
                        //         }
                        //     },
                        // }
                        // }
                        //
                        //         if command_len != command_args.len() as u32 && !potentially_infinite {
                        //             continue;
                        //         }
                        //
                        //         let mut current_node = 0;
                        //         let mut current_arg = 0;
                        //         let mut possible_args = Vec::new();
                        //         println!("executing pathing: {:?} -> {:?} ", path.first().unwrap(), path.last().unwrap());
                        //         loop {
                        //             if current_node != 0 {
                        //                 // the first node in path cannot be product of a redirect
                        //                 // if this node is the product of a redirect, we want to skip it
                        //                 // without this check the tp command would have to be written as
                        //                 // /tp teleport <params> instead of /tp <params>
                        //                 if registry
                        //                     .graph
                        //                     .graph
                        //                     .edges_connecting(path[current_node - 1], path[current_node])
                        //                     .clone()
                        //                     .any(|edge| *edge.weight() == CommandEdgeType::Redirect)
                        //                 {
                        //                     current_node += 1;
                        //                     continue;
                        //                 }
                        //             }
                        //
                        //             println!("---- current_node: {:?} ----", path[current_node]);
                        //
                        //             // check that the executor has the permission to path through this node
                        //             let node_scopes = registry.graph.graph[path[current_node]].scopes.clone();
                        //             if !node_scopes.is_empty() {
                        //                 let mut has_scope = false;
                        //                 for scope in node_scopes {
                        //                     if scope_registry.any_grants(scopes.get(client).unwrap().scopes.clone(), scope) {
                        //                         has_scope = true;
                        //                         break;
                        //                     }
                        //                 }
                        //                 if !has_scope {
                        //                     break;
                        //                 }
                        //             }
                        //
                        //             match &registry.graph.graph[path[current_node]].data {
                        //                 NodeData::Root => {
                        //                     println!("- root");
                        //                 }
                        //                 NodeData::Literal { name } => {
                        //                     let input = match command_args.get(current_arg) {
                        //                         None => {break}
                        //                         Some(arg) => {arg}
                        //                     };
                        //
                        //                     println!("- literal: {}", name);
                        //                     println!("|- command_arg: {}", input);
                        //                     if name != input {
                        //                         break;
                        //                     }
                        //
                        //                     current_arg += 1;
                        //                 }
                        //                 NodeData::Argument { parser, name, .. } => {
                        //                     println!("- argument: {}", name);
                        //                     // let arg_len = parser_len(parser);
                        //
                        //                     let (arg, taken_len) = match parse_arg(&command_args, current_arg, arg_len) {
                        //                         Some(value) => value,
                        //                         None => break,
                        //                     };
                        //
                        //                     if parser_valid_for(parser, arg.clone()) {
                        //                         possible_args.push(arg);
                        //                         println!("|- possible args: {:?}", possible_args);
                        //                         current_arg += taken_len;
                        //                     } else {
                        //                         break;
                        //                     }
                        //                 }
                        //             }
                        //
                        //             if path[current_node] == *leaf && current_arg == command_args.len() {
                        //                 let executable = command.executables.get(&path[current_node]).unwrap();
                        //                 let args = possible_args;
                        //                 let executable = executable(args);
                        //                 println!("executing command with info {:#?}", executable);
                        //                 clients.get_mut(client).unwrap().send_chat_message(
                        //                     format!("executing command with  info {:#?}", executable)
                        //                 );
                        //                 events.send(CommandExecutionEvent {
                        //                     result: executable,
                        //                     executor: client,
                        //                 });
                        //                 break;
                        //             }
                        //             current_node += 1;
                        //         }
                        //     }

                }
            }
        }
    }
}
//
// pub fn parse_arg(parser: &mut ParseInput) -> Option<(String, usize)> {
//     let (arg, taken_len): (String, usize) = match arg_len {
//         ArgLen::Infinite => (
//             command_args.get(current_arg..)?.join(" "),
//             command_args.get(current_arg..)?.len(),
//         ),
//         ArgLen::Exact(num) => (
//             command_args.get(current_arg..current_arg + num as usize)?
//                 .join(" "),
//             num as usize,
//         ),
//         ArgLen::Within(char) => {
//             // example with " char: ""hello world"" will be
//             // ["\"hello", "world\""] in the list we want to get
//             // "hello world".
//
//             if command_args.get(current_arg)?.starts_with(char) {
//                 let mut arg = command_args.get(current_arg)?.clone();
//                 arg.remove(0);
//
//                 // look for a list item that ends with the same char
//                 let mut end_index = current_arg;
//                 for (i, arg) in
//                 command_args.get(current_arg + 1..)?.iter().enumerate()
//                 {
//                     if arg.ends_with(char) {
//                         end_index = i + current_arg + 1;
//                         break;
//                     }
//                 }
//
//                 (
//                     command_args.get(current_arg..end_index + 1)?.join(" "),
//                     (end_index - current_arg + 1),
//                 )
//             } else {
//                 return None;
//             }
//         }
//         ArgLen::WithinExplicit(start, end) => {
//             // example with [ and ] char: "[hello world]" will be
//             // ["[hello", "world]"] in the list we want to get
//             // "hello world".
//
//             if command_args.get(current_arg)?.starts_with(start) {
//                 let mut arg = command_args.get(current_arg)?.clone();
//                 arg.remove(0);
//
//                 // look for a list item that ends with the same char
//                 let mut end_index = current_arg;
//                 for (i, arg) in
//                 command_args.get(current_arg + 1..)?.iter().enumerate()
//                 {
//                     if arg.ends_with(end) {
//                         end_index = i + current_arg + 1;
//                         break;
//                     }
//                 }
//
//                 (
//                     command_args.get(current_arg..end_index + 1)?.join(" "),
//                     (end_index - current_arg + 1),
//                 )
//             } else {
//                 return None;
//             }
//         }
//     };
//     Some((arg, taken_len))
//     None
// }
