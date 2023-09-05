use std::collections::{HashMap, HashSet};
use std::time::Instant;
use bevy_app::{App, Plugin, PreUpdate, Update};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{Added, Commands, EventReader, EventWriter, IntoSystemConfigs, Or, Query, Res};
use bevy_ecs::query::Changed;
use petgraph::Graph;
use petgraph::graph::NodeIndex;
use valence_server::client::{Client, SpawnClientsSet};
use valence_server::event_loop::PacketEvent;
use valence_server::protocol::packets::play::{CommandExecutionC2s, CommandTreeS2c};
use valence_server::protocol::packets::play::command_suggestions_s2c::CommandSuggestionsMatch;
use valence_server::protocol::WritePacket;

use crate::command_graph::{CommandEdgeType, CommandGraph, CommandNode, NodeData};
use crate::command_scopes::CommandScopes;
use crate::{CommandExecutionEvent, CommandProcessedEvent, CommandRegistry, CommandScopeRegistry};
use crate::arg_parser::ParseInput;

pub struct CommandManagerPlugin;

impl Plugin for CommandManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CommandExecutionEvent>()
            .add_event::<CommandProcessedEvent>()
            .add_systems(
            PreUpdate,
            insert_permissions_component.after(SpawnClientsSet),
        )
        .add_systems(Update, (update_command_tree, read_incoming_packets, parse_incoming_commands));

        let graph: CommandGraph = CommandGraph::new();
        let modifiers = HashMap::new();
        let parsers = HashMap::new();
        let executables = HashSet::new();

        app.insert_resource(CommandRegistry { graph, modifiers, parsers, executables });

        app.insert_resource(CommandScopeRegistry::new());
    }
}

pub fn insert_permissions_component(
    mut clients: Query<Entity, Added<Client>>,
    mut commands: Commands,
) {
    for client in clients.iter_mut() {
        println!("Inserting permissions component for client: {:?}", client);
        commands.entity(client).insert(CommandScopes::new());
    }
}

pub fn read_incoming_packets(mut packets: EventReader<PacketEvent>, mut event_writer: EventWriter<CommandExecutionEvent>) {
    for packet in packets.iter() {
        let client = packet.client;
        if let Some(packet) = packet.decode::<CommandExecutionC2s>() {
            event_writer.send(CommandExecutionEvent {
                command: packet.command.to_string(),
                executor: client,
            });
        }
        // if let Some(packet) = packet.decode::<>() {
        //     println!("Received command tree from client: {:?}", packet);
        // }
    }
}

#[allow(clippy::type_complexity)]
pub fn update_command_tree(
    command_registry: Res<CommandRegistry>,
    premission_registry: Res<CommandScopeRegistry>,
    mut new_clients: Query<
        (&mut Client, &CommandScopes),
        Or<(Added<Client>, Changed<CommandScopes>)>,
    >,
) {
    for (mut client, client_permissions) in new_clients.iter_mut() {
        let mut graph = command_registry.graph.clone();
        // trim the graph to only include commands the client has permission to execute
        let mut nodes_to_remove = Vec::new();
        'nodes: for node in graph.graph.node_indices() {
            let node_scopes = &graph.graph[node].scopes;
            if node_scopes.is_empty() {
                continue;
            }
            for permission in node_scopes.iter() {
                if !premission_registry
                    .any_grants(&client_permissions.scopes, permission)
                {
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

fn parse_incoming_commands(
    mut event_reader: EventReader<CommandExecutionEvent>,
    mut event_writer: EventWriter<CommandProcessedEvent>,
    command_registry: Res<CommandRegistry>,
    scope_registry: Res<CommandScopeRegistry>,
    entity_scopes: Query<&CommandScopes>
) {
        for command_event in event_reader.iter() {
            let timer = Instant::now();
            let executor = command_event.executor;
            println!("Received command: {:?}", command_event);
            // theese are the leafs of the graph that are executable under this command group
            let executable_leafs = command_registry.executables.iter().collect::<Vec<&NodeIndex>>();
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
                if command_registry.modifiers.contains_key(&curent_node) {
                    modifiers_to_be_executed.push((curent_node, String::new()));
                }
            }
            // if the node is a literal, we want to match the name of the literal
            // to the input
            NodeData::Literal { name } => {
                match input.match_next(name) {
                    true => {
                        input.pop(); // we want to pop the whitespace after the literal
                        if command_registry.modifiers.contains_key(&curent_node) {
                            modifiers_to_be_executed.push((curent_node, String::new()));
                        }
                    }
                    false => return false,
                }
            }
            // if the node is an argument, we want to parse the argument
            NodeData::Argument { .. } => {

                let parser = match command_registry.parsers.get(&curent_node) {
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
                    if command_registry.modifiers.contains_key(&curent_node) {
                        modifiers_to_be_executed.push((curent_node, input.input[before_cursor..after_cursor].to_string()));
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
    if input.is_done() && executable_leafs.contains(&&curent_node) {
        to_be_executed.push(curent_node);
        return true;
    } else {
        input.cursor = pre_cursor;
    }

    let mut all_invalid = true;
    for neighbor in graph.neighbors(curent_node) {
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
                let edge = graph.find_edge(curent_node, neighbor).unwrap();
                match &graph[edge] {
                    CommandEdgeType::Redirect => {
                        true
                    }
                    _ => false,
                }
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