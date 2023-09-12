use std::collections::{HashMap, HashSet};
use std::time::Instant;

use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{
    Added, Commands, DetectChanges, Event, EventReader, EventWriter, IntoSystemConfigs, Or, Query,
    Res,
};
use bevy_ecs::query::Changed;
use petgraph::graph::NodeIndex;
use petgraph::Graph;
use valence_server::client::{Client, SpawnClientsSet};
use valence_server::event_loop::PacketEvent;
use valence_server::protocol::packets::play::command_tree_s2c::NodeData;
use valence_server::protocol::packets::play::{CommandExecutionC2s, CommandTreeS2c};
use valence_server::protocol::WritePacket;
use valence_server::EventLoopPreUpdate;

use crate::graph::{CommandEdgeType, CommandGraph, CommandNode};
use crate::parsers::ParseInput;
use crate::scopes::CommandScopes;
use crate::{CommandRegistry, CommandScopeRegistry, CommandSystemSet, ModifierValue};

pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CommandExecutionEvent>()
            .add_event::<CommandProcessedEvent>()
            .add_systems(PreUpdate, insert_scope_component.after(SpawnClientsSet))
            .add_systems(
                EventLoopPreUpdate,
                (
                    update_command_tree,
                    command_tree_update_with_client,
                    read_incoming_packets.before(CommandSystemSet),
                    parse_incoming_commands.in_set(CommandSystemSet),
                ),
            );

        let graph: CommandGraph = CommandGraph::new();
        let modifiers = HashMap::new();
        let parsers = HashMap::new();
        let executables = HashSet::new();

        app.insert_resource(CommandRegistry {
            graph,
            modifiers,
            parsers,
            executables,
        });

        app.insert_resource(CommandScopeRegistry::new());
    }
}

/// This event is sent when a command is sent (you can send this with any
/// entity)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Event)]
pub struct CommandExecutionEvent {
    /// the command that was executed eg. "teleport @p 0 ~ 0"
    pub command: String,
    /// usually the Client entity but it could be a command block or something
    /// (whatever the library user wants)
    pub executor: Entity,
}

/// This will only be sent if the command was successfully parsed and an
/// executable was found
#[derive(Debug, Clone, PartialEq, Eq, Event)]
pub struct CommandProcessedEvent {
    /// the command that was executed eg. "teleport @p 0 ~ 0"
    pub command: String,
    /// usually the Client entity but it could be a command block or something
    /// (whatever the library user wants)
    pub executor: Entity,
    /// the modifiers that were applied to the command
    pub modifiers: HashMap<ModifierValue, ModifierValue>,
    /// the node that was executed
    pub node: NodeIndex,
}

fn insert_scope_component(mut clients: Query<Entity, Added<Client>>, mut commands: Commands) {
    for client in clients.iter_mut() {
        commands.entity(client).insert(CommandScopes::new());
    }
}

fn read_incoming_packets(
    mut packets: EventReader<PacketEvent>,
    mut event_writer: EventWriter<CommandExecutionEvent>,
) {
    for packet in packets.iter() {
        let client = packet.client;
        if let Some(packet) = packet.decode::<CommandExecutionC2s>() {
            event_writer.send(CommandExecutionEvent {
                command: packet.command.to_string(),
                executor: client,
            });
        }
    }
}

#[allow(clippy::type_complexity)]
fn command_tree_update_with_client(
    command_registry: Res<CommandRegistry>,
    scope_registry: Res<CommandScopeRegistry>,
    mut new_clients: Query<
        (&mut Client, &CommandScopes),
        Or<(Added<Client>, Changed<CommandScopes>)>,
    >,
) {
    for (mut client, client_scopes) in new_clients.iter_mut() {
        println!("updating command tree for client");
        let time = Instant::now();
        let mut graph = command_registry.graph.clone();
        // trim the graph to only include commands the client has permission to execute
        let mut nodes_to_remove = Vec::new();
        'nodes: for node in graph.graph.node_indices() {
            let node_scopes = &graph.graph[node].scopes;
            if node_scopes.is_empty() {
                continue;
            }
            for scope in node_scopes.iter() {
                if !scope_registry.any_grants(
                    &client_scopes.0.iter().map(|scope| scope.as_str()).collect(),
                    scope,
                ) {
                    // this should be enough to remove the node and all of its children (when it
                    // gets converted into a packet)
                    nodes_to_remove.push(node);
                    continue 'nodes;
                }
            }
        }

        for node in nodes_to_remove {
            graph.graph.remove_node(node);
        }

        println!("converting graph to packet");
        let time2 = Instant::now();
        let packet: CommandTreeS2c = graph.into();
        println!("converting graph to packet took {:?}", time2.elapsed());

        client.write_packet(&packet);
        println!("command tree update took {:?}", time.elapsed());
    }
}

fn update_command_tree(
    command_registry: Res<CommandRegistry>,
    scope_registry: Res<CommandScopeRegistry>,
    mut clients: Query<(&mut Client, &CommandScopes)>,
) {
    if command_registry.is_changed() {
        for (mut client, client_scopes) in clients.iter_mut() {
            let mut graph = command_registry.graph.clone();
            // trim the graph to only include commands the client has permission to execute
            let mut nodes_to_remove = Vec::new();
            'nodes: for node in graph.graph.node_indices() {
                let node_scopes = &graph.graph[node].scopes;
                if node_scopes.is_empty() {
                    continue;
                }
                for scope in node_scopes.iter() {
                    if !scope_registry.any_grants(
                        &client_scopes.0.iter().map(|scope| scope.as_str()).collect(),
                        scope,
                    ) {
                        // this should be enough to remove the node and all of its children (when it
                        // gets converted into a packet)
                        nodes_to_remove.push(node);
                        continue 'nodes;
                    }
                }
            }

            for node in nodes_to_remove {
                graph.graph.remove_node(node);
            }

            let packet: CommandTreeS2c = graph.into();

            client.write_packet(&packet);
        }
    }
}

fn parse_incoming_commands(
    mut event_reader: EventReader<CommandExecutionEvent>,
    mut event_writer: EventWriter<CommandProcessedEvent>,
    command_registry: Res<CommandRegistry>,
    scope_registry: Res<CommandScopeRegistry>,
    entity_scopes: Query<&CommandScopes>,
) {
    for command_event in event_reader.iter() {
        let timer = Instant::now();
        let executor = command_event.executor;
        println!("Received command: {:?}", command_event);
        // these are the leafs of the graph that are executable under this command
        // group
        let executable_leafs = command_registry
            .executables
            .iter()
            .collect::<Vec<&NodeIndex>>();
        println!("Executable leafs: {:?}", executable_leafs);
        let root = command_registry.graph.root;

        let command_input = &command_event.command;
        let graph = &command_registry.graph.graph;
        let mut input = ParseInput::new(command_input);

        let mut to_be_executed = Vec::new();

        let mut args = Vec::new();
        let mut modifiers_to_be_executed = Vec::new();

        parse_command_args(
            &mut args,
            &mut modifiers_to_be_executed,
            &mut input,
            graph,
            &executable_leafs,
            command_registry.as_ref(),
            &mut to_be_executed,
            root,
            executor,
            &entity_scopes,
            scope_registry.as_ref(),
            false,
        );

        let mut modifiers = HashMap::new();
        for (node, modifier) in modifiers_to_be_executed {
            println!("Executing modifier with data: {:?}", modifier);
            command_registry.modifiers.get(&node).unwrap()(modifier, &mut modifiers);
        }

        for node in to_be_executed {
            println!("Executing command with data: {:?}", args);
            println!("Executing command with modifiers: {:?}", modifiers);
            event_writer.send(CommandProcessedEvent {
                command: args.join(" "),
                executor,
                modifiers: modifiers.clone(),
                node,
            });
        }

        println!("Command processed in: {:?}", timer.elapsed());
    }
}

#[allow(clippy::too_many_arguments)]
/// recursively parse the command args.
fn parse_command_args(
    command_args: &mut Vec<String>,
    modifiers_to_be_executed: &mut Vec<(NodeIndex, String)>,
    input: &mut ParseInput,
    graph: &Graph<CommandNode, CommandEdgeType>,
    executable_leafs: &[&NodeIndex],
    command_registry: &CommandRegistry,
    to_be_executed: &mut Vec<NodeIndex>,
    current_node: NodeIndex,
    executor: Entity,
    scopes: &Query<&CommandScopes>,
    scope_registry: &CommandScopeRegistry,
    coming_from_redirect: bool,
) -> bool {
    let node_scopes = &graph[current_node].scopes;
    let default_scopes = CommandScopes::new();
    let client_scopes: Vec<&str> = scopes
        .get(executor)
        .unwrap_or(&default_scopes)
        .0
        .iter()
        .map(|scope| scope.as_str())
        .collect();
    // if empty, we assume the node is global
    if !node_scopes.is_empty() {
        let mut has_scope = false;
        for scope in node_scopes {
            if scope_registry.any_grants(&client_scopes, scope) {
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
        match &graph[current_node].data {
            // no real need to check for root node
            NodeData::Root => {
                if command_registry.modifiers.contains_key(&current_node) {
                    modifiers_to_be_executed.push((current_node, String::new()));
                }
            }
            // if the node is a literal, we want to match the name of the literal
            // to the input
            NodeData::Literal { name } => {
                match input.match_next(name) {
                    true => {
                        input.pop(); // we want to pop the whitespace after the literal
                        if command_registry.modifiers.contains_key(&current_node) {
                            modifiers_to_be_executed.push((current_node, String::new()));
                        }
                    }
                    false => return false,
                }
            }
            // if the node is an argument, we want to parse the argument
            NodeData::Argument { .. } => {
                let parser = match command_registry.parsers.get(&current_node) {
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
                    command_args.push(input.input[before_cursor..after_cursor].to_string());
                    if command_registry.modifiers.contains_key(&current_node) {
                        modifiers_to_be_executed.push((
                            current_node,
                            input.input[before_cursor..after_cursor].to_string(),
                        ));
                    }
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
    if input.is_done() && executable_leafs.contains(&&current_node) {
        to_be_executed.push(current_node);
        return true;
    } else {
        input.cursor = pre_cursor;
    }

    let mut all_invalid = true;
    for neighbor in graph.neighbors(current_node) {
        let pre_cursor = input.cursor;
        let mut args = command_args.clone();
        let mut modifiers = modifiers_to_be_executed.clone();
        let valid = parse_command_args(
            &mut args,
            &mut modifiers,
            input,
            graph,
            executable_leafs,
            command_registry,
            to_be_executed,
            neighbor,
            executor,
            scopes,
            scope_registry,
            {
                let edge = graph.find_edge(current_node, neighbor).unwrap();
                matches!(&graph[edge], CommandEdgeType::Redirect)
            },
        );
        if valid {
            *command_args = args;
            *modifiers_to_be_executed = modifiers;
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
