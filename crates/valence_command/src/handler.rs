use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;

use bevy_app::{App, Plugin, PostStartup, Update};
use bevy_ecs::change_detection::{Res, ResMut};
use bevy_ecs::event::{Event, EventReader, EventWriter};
use bevy_ecs::prelude::{Entity, Resource};
use bevy_ecs::system::Query;

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
        app.add_event::<CommandResultEvent<T>>()
            .insert_resource(CommandResource::<T>::new())
            .add_systems(Update, command_event_system::<T>)
            .add_systems(PostStartup, command_startup_system::<T>);
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

        let command_input = &command_event.command;
        let graph = &registry.graph.graph;
        let mut input = ParseInput::new(command_input);

        let mut to_be_executed = Vec::new();

        let mut args = Vec::new();

        parse_command_args(
            &mut args,
            &mut input,
            graph,
            &executable_leafs,
            &command.parsers,
            &mut to_be_executed,
            root,
            executor,
            &scopes,
            scope_registry.as_ref(),
            false,
        );

            for executable in to_be_executed {
                trace!("Executing command with args: {:?}", args);
                let result =
                    command.executables.get(&executable).unwrap()(&mut ParseInput::new(&args.join(" ")));
                trace!("executing command with info {:#?}", result);
                events.send(CommandResultEvent { result, executor });
            }
    }
}

#[allow(clippy::too_many_arguments)]
/// recursively parse the command args.
fn parse_command_args(
    command_args: &mut Vec<String>,
    input: &mut ParseInput,
    graph: &Graph<CommandNode, CommandEdgeType>,
    executable_leafs: &[&NodeIndex],
    parsers: &HashMap<NodeIndex, fn(&mut ParseInput) -> bool>,
    to_be_executed: &mut Vec<NodeIndex>,
    curent_node: NodeIndex,
    executor: Entity,
    scopes: &Query<&CommandScopes>,
    scope_registry: &CommandScopeRegistry,
    coming_from_redirect: bool,
) -> bool {
    let node_scopes = &graph[curent_node].scopes;
    let default_scopes = CommandScopes::new();
    let client_scopes = &scopes.get(executor).unwrap_or(&default_scopes).scopes;
    // if empty, we assume the node is global
    if !node_scopes.is_empty() {
        let mut has_scope = false;
        for scope in node_scopes {
            if scope_registry.any_grants(client_scopes, scope) {
                has_scope = true;
                break;
            }
        }
        if !has_scope {
            return false;
        }
    }

    if !coming_from_redirect {
        // we want to skip whitespace before matching the node
        input.skip_whitespace();
        match &graph[curent_node].data {
            // no real need to check for root node
            NodeData::Root => {
                println!("root: {:?}", graph[curent_node].data);
            }
            // if the node is a literal, we want to match the name of the literal
            // to the input
            NodeData::Literal { name } => {
                println!("literal: {:?}", graph[curent_node].data);
                match input.match_next(name) {
                    true => {}
                    false => return false,
                }
            }
            // if the node is an argument, we want to parse the argument
            NodeData::Argument { .. } => {
                println!("argument: {:?}", graph[curent_node].data);

                let parser = match parsers.get(&curent_node) {
                    Some(parser) => parser,
                    None => {
                        return false;
                    }
                };
                // we want to save the cursor position before and after parsing
                // this is so we can save the argument to the command args
                let before_cursor = input.cursor;
                let valid = parser(input);
                let after_cursor = input.cursor;
                if valid {
                    command_args.push(input.input[before_cursor..after_cursor].to_string())
                } else {
                    return false;
                }
            }
        }
    } else {
        command_args.clear();
    }


    let pre_cursor = input.cursor;
    input.skip_whitespace();
    if input.is_done() && executable_leafs.contains(&&curent_node){
        to_be_executed.push(curent_node);
        return true;
    } else {
        input.cursor = pre_cursor;
    }


    let mut all_invalid = true;
    for neighbor in graph.neighbors(curent_node) {
        let pre_cursor = input.cursor;
        let mut args = command_args.clone();
        let valid = parse_command_args(
            &mut args,
            input,
            graph,
            executable_leafs,
            parsers,
            to_be_executed,
            neighbor,
            executor,
            scopes,
            scope_registry,
            {
                let edge = graph.find_edge(curent_node, neighbor).unwrap();
                match &graph[edge] {
                    CommandEdgeType::Redirect => {
                        println!("redirecting to: {:?}", graph[neighbor].data);
                        true
                    }
                    _ => false,
                }
            },
        );
        if valid {
            println!("args2: {:?}", args);
            *command_args = args;
            all_invalid = false;
            break;
        } else {
            input.cursor = pre_cursor;
        }
    }
    if all_invalid {
        return false;
    }
    true
}
