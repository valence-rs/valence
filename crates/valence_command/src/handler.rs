use std::collections::HashMap;

use bevy_app::{App, Plugin, PostStartup, Update};
use bevy_ecs::change_detection::{Res, ResMut};
use bevy_ecs::event::{Event, EventReader, EventWriter};
use bevy_ecs::prelude::{Entity, Resource};
use petgraph::algo::all_simple_paths;
use petgraph::dot::Dot;
use petgraph::prelude::NodeIndex;
use petgraph::Graph;
use valence_client::event_loop::PacketEvent;



use crate::arg_parser::{ArgLen};
use crate::command_graph::{
    CommandEdgeType, CommandGraphBuilder, CommandNode, NodeData,
};

use crate::{packet, Command, CommandRegistry, CommandTypingEvent};

pub struct CommandHandler<T>
where
    T: Command + Resource + Clone,
{
    command: T,
}

impl<T> CommandHandler<T>
where
    T: Command + Resource + Clone,
{
    pub fn from_command(command: T) -> Self {
        CommandHandler { command }
    }

    pub fn command_name() -> String {
        T::name()
    }
}

#[derive(Resource)]
struct CommandResource<T: Command> {
    command: T,
    executables: HashMap<NodeIndex, fn(Vec<String>) -> T::CommandExecutables>,
}

impl<T: Command> CommandResource<T> {
    pub fn new(command: T) -> Self {
        CommandResource {
            command,
            executables: HashMap::new(),
        }
    }
}

#[derive(Event)]
pub struct CommandExecutionEvent<T>
where
    T: Command,
    T::CommandExecutables: Send + Sync + 'static,
{
    pub result: T::CommandExecutables,
    pub executor: Entity,
}

impl<T> Plugin for CommandHandler<T>
where
    T: Command + Resource + Clone,
{
    fn build(&self, app: &mut App) {
        println!("Registering command: {}", Self::command_name());

        app.add_event::<CommandTypingEvent<T>>()
            .add_event::<CommandExecutionEvent<T>>()
            .insert_resource(CommandResource::new(self.command.clone()))
            .add_systems(Update, command_event_system::<T>)
            .add_systems(PostStartup, command_startup_system::<T>);

        println!("Registered command: {}", Self::command_name());
    }
}

fn command_startup_system<T>(
    mut registry: ResMut<CommandRegistry>,
    mut command: ResMut<CommandResource<T>>,
) where
    T: Command + Resource + Clone,
{
    let mut executables = HashMap::new();
    let graph_builder = &mut CommandGraphBuilder::new(&mut registry, &mut executables);
    command.command.assemble_graph(graph_builder);

    println!("Command graph: {}", &registry.graph);
    command.executables.extend(executables);
}

/// this system reads incoming command events and prints them to the console
fn command_event_system<T>(
    mut packets: EventReader<PacketEvent>,
    registry: Res<CommandRegistry>,
    mut events: EventWriter<CommandExecutionEvent<T>>,
    command: ResMut<CommandResource<T>>,
) where
    T: Command + Resource,
    T::CommandExecutables: Send + Sync,
{
    for packet in packets.iter() {
        let client = packet.client;
        if let Some(packet) = packet.decode::<packet::CommandExecutionC2s>() {
            println!("Received command: {:?}", packet);
            let executable_leafs = command.executables.keys().collect::<Vec<&NodeIndex>>();
            println!("Executable leafs: {:?}", executable_leafs);
            let root = registry.graph.root;
            for leaf in executable_leafs {
                // we want to find all possible paths from root to leaf then check if the path
                // matches the command

                println!("{}", Dot::new(&registry.graph.graph));

                let mut paths = all_simple_paths::<
                    Vec<NodeIndex>,
                    &Graph<CommandNode, CommandEdgeType>,
                >(&registry.graph.graph, root, *leaf, 0, None);

                let command_args = packet
                    .command
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>();
                while let Some(path) = paths.next() {
                    let mut potentially_infinite = false;

                    let mut command_len = 0;

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

                        match &registry.graph.graph[*node].data {
                            NodeData::Root => {}
                            NodeData::Literal { .. } => command_len += 1,
                            NodeData::Argument { parser, .. } => match parser.len() {
                                ArgLen::Infinite => {
                                    potentially_infinite = true;
                                    command_len = 0;
                                }
                                ArgLen::Exact(num) => command_len += num,
                                ArgLen::Within(_) => {
                                    potentially_infinite = true;
                                    command_len = 0;
                                }
                                ArgLen::WithinExplicit(..) => {
                                    potentially_infinite = true;
                                    command_len = 0;
                                }
                            },
                        }
                    }

                    if command_len != command_args.len() as u32 && !potentially_infinite {
                        continue;
                    }

                    let mut current_node = 0;
                    let mut current_arg = 0;
                    let mut possible_args = Vec::new();
                    loop {
                        if current_node != 0 {
                            // the first node in path cannot be product of a redirect
                            // if this node is the product of a redirect, we want to skip it
                            // without this check the tp command would have to be written as
                            // /tp teleport <params> instead of /tp <params>
                            if registry
                                .graph
                                .graph
                                .edges_connecting(path[current_node - 1], path[current_node])
                                .clone()
                                .any(|edge| *edge.weight() == CommandEdgeType::Redirect)
                            {
                                current_node += 1;
                                continue;
                            }
                        }

                        println!("current_node: {}", current_node);

                        // check that the executor has the permission to execute this command


                        match &registry.graph.graph[path[current_node]].data {
                            NodeData::Root => {
                                println!("root");
                            }
                            NodeData::Literal { name } => {
                                println!("literal: {}", name);
                                println!("command_arg: {}", command_args[current_arg]);
                                if name != &command_args[current_arg] {
                                    break;
                                }

                                current_arg += 1;
                            }
                            NodeData::Argument { parser, name, .. } => {
                                println!("argument: {}", name);
                                let arg_len = parser.len();

                                let (arg, taken_len): (String, usize) = match arg_len {
                                    ArgLen::Infinite => (
                                        command_args[current_arg..].join(" "),
                                        command_args[current_arg..].len(),
                                    ),
                                    ArgLen::Exact(num) => (
                                        command_args[current_arg..current_arg + num as usize]
                                            .join(" "),
                                        num as usize,
                                    ),
                                    ArgLen::Within(char) => {
                                        // example with " char: ""hello world"" will be
                                        // ["\"hello", "world\""] in the list we want to get
                                        // "hello world".

                                        if command_args[current_arg].starts_with(char) {
                                            let mut arg = command_args[current_arg].clone();
                                            arg.remove(0);

                                            // look for a list item that ends with the same char
                                            let mut end_index = current_arg;
                                            for (i, arg) in
                                                command_args[current_arg + 1..].iter().enumerate()
                                            {
                                                if arg.ends_with(char) {
                                                    end_index = i + current_arg + 1;
                                                    break;
                                                }
                                            }

                                            (
                                                command_args[current_arg..end_index + 1].join(" "),
                                                (end_index - current_arg + 1),
                                            )
                                        } else {
                                            break;
                                        }
                                    }
                                    ArgLen::WithinExplicit(start, end) => {
                                        // example with [ and ] char: "[hello world]" will be
                                        // ["[hello", "world]"] in the list we want to get
                                        // "hello world".

                                        if command_args[current_arg].starts_with(start) {
                                            let mut arg = command_args[current_arg].clone();
                                            arg.remove(0);

                                            // look for a list item that ends with the same char
                                            let mut end_index = current_arg;
                                            for (i, arg) in
                                                command_args[current_arg + 1..].iter().enumerate()
                                            {
                                                if arg.ends_with(end) {
                                                    end_index = i + current_arg + 1;
                                                    break;
                                                }
                                            }

                                            (
                                                command_args[current_arg..end_index + 1].join(" "),
                                                (end_index - current_arg + 1),
                                            )
                                        } else {
                                            break;
                                        }
                                    }
                                };

                                if parser.valid_for(arg.clone()) {
                                    possible_args.push(arg);
                                    println!("possible args: {:?}", possible_args);
                                    current_arg += taken_len;
                                } else {
                                    break;
                                }
                            }
                        }

                        if path[current_node] == *leaf {
                            let executable = command.executables.get(&path[current_node]).unwrap();
                            let args = possible_args;
                            let executable = executable(args);
                            events.send(CommandExecutionEvent {
                                result: executable,
                                executor: client,
                            });
                            break;
                        }
                        current_node += 1;
                    }
                }
            }
        }
    }
}
