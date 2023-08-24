use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;

use bevy_app::{App, Plugin, PostStartup, Update};
use bevy_ecs::change_detection::{Res, ResMut};
use bevy_ecs::event::{Event, EventReader, EventWriter};
use bevy_ecs::prelude::{Entity, Resource};
use bevy_ecs::system::Query;
use petgraph::algo::all_simple_paths;

use petgraph::prelude::NodeIndex;
use petgraph::Graph;
use tracing::trace;


use crate::arg_parser::ParseInput;
use crate::command_graph::{CommandEdgeType, CommandGraphBuilder, CommandNode, NodeData};
use crate::command_scopes::CommandScopes;
use crate::{Command, CommandExecutionEvent, CommandRegistry, CommandScopeRegistry};

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
pub struct CommandResultEvent<T>
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

        app.add_event::<CommandResultEvent<T>>()
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
    mut commands_executed: EventReader<CommandExecutionEvent>,
    registry: Res<CommandRegistry>,
    mut events: EventWriter<CommandResultEvent<T>>,
    command: ResMut<CommandResource<T>>,
    scope_registry: Res<CommandScopeRegistry>,
    scopes: Query<&CommandScopes>,
) where
    T: Command + Send + Sync + Debug,
{
    for command_event in commands_executed.iter() {
        let executor = command_event.executor;
        trace!("Received command: {:?}", command_event);
        // theese are the leafs of the graph that are executable under this command group
        let executable_leafs = command.executables.keys().collect::<Vec<&NodeIndex>>();
        trace!("Executable leafs: {:?}", executable_leafs);
        let root = registry.graph.root;
        for leaf in executable_leafs {
            // we want to find all possible paths from root to leaf then check if the path
            // matches the command
            let paths = all_simple_paths::<Vec<NodeIndex>, &Graph<CommandNode, CommandEdgeType>>(
                &registry.graph.graph,
                root,
                *leaf,
                0,
                None,
            );

            let graph = &registry.graph.graph;
            let command_input = &command_event.command;
            let mut command_args = Vec::new();
            'paths: for path in paths {
                let mut input = ParseInput::new(command_input);
                for (i, node) in path.iter().enumerate() {
                    let node_scopes = &graph[path[i]].scopes;
                    let default_scopes = CommandScopes::new();
                    let client_scopes =
                        &scopes.get(executor).unwrap_or(&default_scopes).scopes;
                    if !node_scopes.is_empty() {
                        // if empty, we assume the node is global
                        let mut has_scope = false;
                        for scope in node_scopes {
                            if scope_registry.any_grants(client_scopes, scope) {
                                has_scope = true;
                                break;
                            }
                        }
                        if !has_scope {
                            break;
                        }
                    }

                    if i != 0 {
                        // the first node in path cannot be product of a redirect
                        // if this node is the product of a redirect, we want to skip it
                        // without this check the tp command would have to be written as
                        // /tp teleport <params> instead of /tp <params>
                        if graph
                            .edges_connecting(path[i - 1], path[i])
                            .clone()
                            .any(|edge| *edge.weight() == CommandEdgeType::Redirect)
                        {
                            continue;
                        }
                    }

                    // we want to skip whitespace before matching the node
                    input.skip_whitespace();
                    match &graph[*node].data {
                        // no real need to check for root node
                        NodeData::Root => {}
                        // if the node is a literal, we want to match the name of the literal
                        // to the input
                        NodeData::Literal { name } => match input.match_next(name) {
                            true => {}
                            false => continue 'paths,
                        },
                        // if the node is an argument, we want to parse the argument
                        NodeData::Argument { .. } => {
                            let parser = command.parsers.get(node).unwrap();
                            // we want to save the cursor position before and after parsing
                            // this is so we can save the argument to the command args
                            // or reset the cursor if the argument is invalid
                            let before_cursor = input.cursor;
                            let valid = parser(&mut input);
                            let after_cursor = input.cursor;
                            if valid {
                                command_args
                                    .push(input.input[before_cursor..after_cursor].to_string());
                            } else {
                                input.set_cursor(before_cursor);
                                continue 'paths;
                            }
                        }
                    }
                }
                if input.cursor == input.input.len() {
                    let mut command_args = ParseInput::new(command_args.join(" "));
                    trace!("Executing command with args: {:?}", command_args);
                    let result =
                        command.executables.get(&path[path.len() - 1]).unwrap()(&mut command_args);
                    trace!("executing command with info {:#?}", result);
                    events.send(CommandResultEvent { result, executor });

                    break 'paths;
                }
            }
        }
    }
}
