use std::collections::{HashMap, HashSet};

use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{
    Added, Changed, Commands, DetectChanges, Event, EventReader, EventWriter, IntoSystemConfigs,
    Mut, Or, Query, Res,
};
use petgraph::graph::NodeIndex;
use petgraph::prelude::EdgeRef;
use petgraph::{Direction, Graph};
use tracing::{debug, warn};
use valence_server::client::{Client, SpawnClientsSet};
use valence_server::event_loop::PacketEvent;
use valence_server::protocol::packets::play::command_tree_s2c::NodeData;
use valence_server::protocol::packets::play::{CommandExecutionC2s, CommandTreeS2c};
use valence_server::protocol::WritePacket;
use valence_server::EventLoopPreUpdate;

use crate::graph::{CommandEdgeType, CommandGraph, CommandNode};
use crate::parsers::ParseInput;
use crate::scopes::{CommandScopePlugin, CommandScopes};
use crate::{CommandRegistry, CommandScopeRegistry, CommandSystemSet, ModifierValue};

pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CommandScopePlugin)
            .add_event::<CommandExecutionEvent>()
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
    clients: Query<&Client>,
    mut command_execution_events: EventWriter<CommandExecutionEvent>,
) {
    for packet in packets.read() {
        let Some(pkt) = packet.decode::<CommandExecutionC2s>() else {
            continue;
        };

        if !clients.contains(packet.client) {
            continue;
        }

        command_execution_events.send(CommandExecutionEvent {
            command: pkt.command.to_string(),
            executor: packet.client,
        });
    }
}

#[allow(clippy::type_complexity)]
fn command_tree_update_with_client(
    command_registry: Res<CommandRegistry>,
    scope_registry: Res<CommandScopeRegistry>,
    mut updated_clients: Query<
        (&mut Client, &CommandScopes),
        Or<(Added<Client>, Changed<CommandScopes>)>,
    >,
) {
    update_client_command_tree(
        &command_registry,
        scope_registry,
        &mut updated_clients.iter_mut().collect(),
    );
}

fn update_command_tree(
    command_registry: Res<CommandRegistry>,
    scope_registry: Res<CommandScopeRegistry>,
    mut clients: Query<(&mut Client, &CommandScopes)>,
) {
    if command_registry.is_changed() {
        update_client_command_tree(
            &command_registry,
            scope_registry,
            &mut clients.iter_mut().collect(),
        );
    }
}

fn update_client_command_tree(
    command_registry: &Res<CommandRegistry>,
    scope_registry: Res<CommandScopeRegistry>,
    updated_clients: &mut Vec<(Mut<Client>, &CommandScopes)>,
) {
    for (ref mut client, client_scopes) in updated_clients {
        let time = std::time::Instant::now();

        let old_graph = &command_registry.graph;
        let mut new_graph = Graph::new();

        // collect a new graph into only nodes that are allowed to be executed
        let root = old_graph.root;

        let mut to_visit = vec![(None, root)];
        let mut already_visited = HashSet::new(); // prevent recursion
        let mut old_to_new = HashMap::new();
        let mut new_root = None;

        while let Some((parent, node)) = to_visit.pop() {
            if already_visited.contains(&(parent.map(|(_, edge)| edge), node)) {
                continue;
            }
            already_visited.insert((parent.map(|(_, edge)| edge), node));
            let node_scopes = &old_graph.graph[node].scopes;
            if !node_scopes.is_empty() {
                let mut has_scope = false;
                for scope in node_scopes {
                    if scope_registry.any_grants(
                        &client_scopes.0.iter().map(|scope| scope.as_str()).collect(),
                        scope,
                    ) {
                        has_scope = true;
                        break;
                    }
                }
                if !has_scope {
                    continue;
                }
            }

            let new_node = *old_to_new
                .entry(node)
                .or_insert_with(|| new_graph.add_node(old_graph.graph[node].clone()));

            for neighbor in old_graph.graph.edges_directed(node, Direction::Outgoing) {
                to_visit.push((Some((new_node, neighbor.weight())), neighbor.target()));
            }

            if let Some(parent) = parent {
                new_graph.add_edge(parent.0, new_node, *parent.1);
            } else {
                new_root = Some(new_node);
            }
        }

        match new_root {
            Some(new_root) => {
                let command_graph = CommandGraph {
                    graph: new_graph,
                    root: new_root,
                };
                let packet: CommandTreeS2c = command_graph.into();

                client.write_packet(&packet);
            }
            None => {
                warn!(
                    "Client has no permissions to execute any commands so we sent them nothing. \
                     It is generally a bad idea to scope the root node of the command graph as it \
                     can cause undefined behavior. For example, if the player has permission to \
                     execute a command before you change the scope of the root node, the packet \
                     will not be sent to the client and so the client will still think they can \
                     execute the command."
                )
            }
        }

        debug!("command tree update took {:?}", time.elapsed());
    }
}

fn parse_incoming_commands(
    mut command_execution_events: EventReader<CommandExecutionEvent>,
    mut command_processed_events: EventWriter<CommandProcessedEvent>,
    command_registry: Res<CommandRegistry>,
    scope_registry: Res<CommandScopeRegistry>,
    entity_scopes: Query<&CommandScopes>,
) {
    for command_execution in command_execution_events.read() {
        let executor = command_execution.executor;
        // these are the leafs of the graph that are executable under this command
        // group
        let executable_leafs = command_registry
            .executables
            .iter()
            .collect::<Vec<&NodeIndex>>();
        let root = command_registry.graph.root;

        let command_input = &*command_execution.command;
        let graph = &command_registry.graph.graph;
        let input = ParseInput::new(command_input);

        let mut to_be_executed = Vec::new();

        let mut args = Vec::new();
        let mut modifiers_to_be_executed = Vec::new();

        parse_command_args(
            &mut args,
            &mut modifiers_to_be_executed,
            input,
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
            command_registry.modifiers.get(&node).unwrap()(modifier, &mut modifiers);
        }

        debug!("Command processed: /{}", command_execution.command);

        for node in to_be_executed {
            println!("executing node: {:?}", node);
            command_processed_events.send(CommandProcessedEvent {
                command: args.join(" "),
                executor,
                modifiers: modifiers.clone(),
                node,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
/// recursively parse the command args.
fn parse_command_args(
    command_args: &mut Vec<String>,
    modifiers_to_be_executed: &mut Vec<(NodeIndex, String)>,
    mut input: ParseInput,
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
                        if !input.match_next(" ") && !input.is_done() {
                            return false;
                        } // we want to pop the whitespace after the literal
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
                // we want to save the input before and after parsing
                // this is so we can save the argument to the command args
                let pre_input = input.clone().into_inner();
                let valid = parser(&mut input);
                if valid {
                    // If input.len() > pre_input.len() the parser replaced the input
                    let Some(arg) = pre_input
                        .get(..pre_input.len().wrapping_sub(input.len()))
                        .map(|s| s.to_string())
                    else {
                        panic!(
                            "Parser replaced input with another string. This is not allowed. \
                             Attempting to parse: {}",
                            input.into_inner()
                        );
                    };

                    if command_registry.modifiers.contains_key(&current_node) {
                        modifiers_to_be_executed.push((current_node, arg.clone()));
                    }
                    command_args.push(arg);
                } else {
                    return false;
                }
            }
        }
    } else {
        command_args.clear();
    }

    input.skip_whitespace();
    if input.is_done() && executable_leafs.contains(&&current_node) {
        to_be_executed.push(current_node);
        return true;
    }

    let mut all_invalid = true;
    for neighbor in graph.neighbors(current_node) {
        let pre_input = input.clone();
        let mut args = command_args.clone();
        let mut modifiers = modifiers_to_be_executed.clone();
        let valid = parse_command_args(
            &mut args,
            &mut modifiers,
            input.clone(),
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
        } else {
            input = pre_input;
        }
    }
    if all_invalid {
        return false;
    }
    true
}
