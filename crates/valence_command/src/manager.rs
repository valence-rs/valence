use std::collections::HashMap;
use bevy_app::{App, Plugin, PreUpdate, Update};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{Added, Commands, EventReader, EventWriter, IntoSystemConfigs, Or, Query, Res};
use bevy_ecs::query::Changed;
use valence_server::client::{Client, SpawnClientsSet};
use valence_server::event_loop::PacketEvent;
use valence_server::protocol::packets::play::{CommandExecutionC2s, CommandTreeS2c};
use valence_server::protocol::WritePacket;

use crate::command_graph::CommandGraph;
use crate::command_scopes::CommandScopes;
use crate::{CommandExecutionEvent, CommandRegistry, CommandScopeRegistry};

pub struct CommandManagerPlugin;

impl Plugin for CommandManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CommandExecutionEvent>()
            .add_systems(
            PreUpdate,
            insert_permissions_component.after(SpawnClientsSet),
        )
        .add_systems(Update, (update_command_tree, read_incoming_packets));

        let graph: CommandGraph = CommandGraph::new();
        let modifiers = HashMap::new();
        let parsers = HashMap::new();

        app.insert_resource(CommandRegistry { graph, modifiers, parsers });

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
