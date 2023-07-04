use std::borrow::Cow;

use bevy_ecs::prelude::{Bundle, Component, DetectChanges, Entity};
use bevy_ecs::query::{Added, Changed, Or, With};
use bevy_ecs::system::Query;
use bevy_ecs::world::Ref;
use valence_client::Client;
use valence_core::protocol::encode::WritePacket;
use valence_core::protocol::packet::command::{Node, NodeData};
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::Encode;
use valence_core::scratch::ScratchBuf;

use crate::packet::{CommandTreeRawNodes, CommandTreeS2c};

#[derive(Debug, Component)]
pub struct Nodes {
    bytes: Vec<u8>,
    root_children: Vec<VarInt>,
    count: usize,
}

pub const ROOT_ID: VarInt = VarInt(0);

impl Nodes {
    pub fn new() -> Self {
        Self {
            bytes: vec![],
            root_children: vec![],
            count: 1,
        }
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn insert_command_nodes<'a>(
        &mut self,
        mut nodes: impl Iterator<Item = Node<'a>>,
    ) -> anyhow::Result<()> {
        if let Some(node) = nodes.next() {
            self.root_children.push(VarInt(self.count as i32));
            self.count += 1;
            node.encode(&mut self.bytes)?;
        }

        for node in nodes {
            self.count += 1;
            node.encode(&mut self.bytes)?;
        }
        Ok(())
    }

    pub(crate) fn to_pkt(&self) -> CommandTreeS2c<CommandTreeRawNodes> {
        CommandTreeS2c {
            nodes: CommandTreeRawNodes {
                count: VarInt(self.count() as i32),
                root: Node {
                    children: Cow::Borrowed(&self.root_children),
                    data: NodeData::Root,
                    executable: false,
                    redirect_node: None,
                },
                bytes: &self.bytes,
            },
            root_index: ROOT_ID,
        }
    }
}

impl Default for Nodes {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Component)]
pub struct PrimaryNodes;

#[derive(Component, PartialEq)]
pub struct ClientNode(pub Entity);

pub fn update_nodes(
    mut client_query: Query<(Option<&ClientNode>, &mut Client)>,
    node_query: Query<(Entity, &Nodes, Option<&PrimaryNodes>), Changed<Nodes>>,
) {
    for (entity, nodes, is_primary_node) in node_query.iter() {
        let expected_client_node = is_primary_node.map(|_| ClientNode(entity));

        let pkt = nodes.to_pkt();

        for (client_node, mut client) in client_query.iter_mut() {
            if expected_client_node.as_ref() == client_node {
                client.write_packet(&pkt);
            }
        }
    }
}

pub fn update_client_nodes(
    mut client_query: Query<
        (Option<&ClientNode>, &mut Client),
        Or<(Changed<ClientNode>, Added<Client>)>,
    >,
    node_query: Query<Ref<Nodes>>,
    primary_node_query: Query<Ref<Nodes>, With<PrimaryNodes>>,
) {
    for (client_node, mut client) in client_query.iter_mut() {
        let nodes = match client_node {
            Some(entity) => node_query
                .get(entity.0)
                .expect("Expected ClientNode targets nodes"),
            None => primary_node_query
                .get_single()
                .expect("Expected single primary nodes"),
        };

        if nodes.is_changed() {
            continue;
        }

        client.write_packet(&nodes.to_pkt())
    }
}
