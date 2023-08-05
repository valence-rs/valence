

use bevy_app::{App, Plugin, PreUpdate, Update};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{
    Added, Commands, IntoSystemConfigs, Or, Query, Res,
};
use bevy_ecs::query::Changed;



use valence_client::{Client, SpawnClientsSet};
use valence_core::protocol::encode::WritePacket;

use crate::command_graph::{CommandGraph};
use crate::packet::CommandTreeS2c;
use crate::command_scopes::CommandScopes;
use crate::{CommandRegistry, CommandScopeRegistry};

pub struct CommandManagerPlugin;

impl Plugin for CommandManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            insert_permissions_component.after(SpawnClientsSet),
        )
        .add_systems(Update, update_command_tree);

        let graph: CommandGraph = CommandGraph::new();

        app.insert_resource(CommandRegistry { graph });

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

#[allow(clippy::type_complexity)]
pub fn update_command_tree(
    command_registry: Res<CommandRegistry>,
    premission_registry: Res<CommandScopeRegistry>,
    mut new_clients: Query<(&mut Client, &CommandScopes), Or<(Added<Client>, Changed<CommandScopes>)>>,
) {
    for (mut client, client_permissions) in new_clients.iter_mut() {
        let mut graph = command_registry.graph.clone();
        // trim the graph to only include commands the client has permission to execute
        let mut nodes_to_remove = Vec::new();
        'nodes: for node in graph.graph.node_indices() {
            let node_scopes = &graph.graph[node].scopes;
            for permission in node_scopes.iter() {
                if !premission_registry
                    .any_grants(client_permissions.scopes.clone(), permission.clone())
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

// pub fn handle_change_command_graph(mut registry: Res<CommandRegistry>, mut
// clients) {     for packet in packets.iter() {
//         if let Some(packet) =
// packet.decode::<packet::ChangeCommandGraphS2c>() {             registry.graph
// = packet.graph.into();         }
//     }
// }
