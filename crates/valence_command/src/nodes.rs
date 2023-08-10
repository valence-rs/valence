use std::borrow::Cow;
use std::cell::UnsafeCell;
use std::collections::HashMap;

use bevy_ecs::prelude::{Component, DetectChanges};
use bevy_ecs::query::{Added, Changed, Or, WorldQuery};
use bevy_ecs::system::{Local, ParamSet, Query, Res, ResMut, Resource, System};
use bevy_ecs::world::Ref;
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use valence_client::Client;
use valence_core::__private::VarInt;
use valence_core::protocol::encode::WritePacket;
use valence_core::protocol::Encode;

use crate::command::CommandArguments;
use crate::parse::ParseObject;
use crate::pkt::{self, RawCommandTreeS2c};

#[derive(Resource)]
pub struct NodeGraphInWorld(pub Option<NodeGraph>);

impl Default for NodeGraphInWorld {
    fn default() -> Self {
        Self(Some(NodeGraph::default()))
    }
}

impl NodeGraphInWorld {
    const MESSAGE: &str =
        "This NodeGraph is used by some system, which is requiring full access to it.";

    /// # Panics
    /// If this method were called in an environment, which requires full access
    /// to it
    pub fn get(&self) -> &NodeGraph {
        self.0.as_ref().expect(Self::MESSAGE)
    }

    /// # Panics
    /// If this method were called in an environment, which requires full access
    /// to it
    pub fn get_mut(&mut self) -> &mut NodeGraph {
        self.0.as_mut().expect(Self::MESSAGE)
    }

    pub(crate) fn take(&mut self) -> NodeGraph {
        std::mem::replace(&mut self.0, None).expect(Self::MESSAGE)
    }

    pub(crate) fn insert(&mut self, graph: NodeGraph) {
        assert!(self.0.is_none(), "This NodeGraph is in the world");
        self.0 = Some(graph);
    }
}

pub struct NodeGraph {
    pub(crate) shared: UnsafeCell<SharedNodeGraph>,
    pub(crate) changed: bool,
    pub(crate) root_nodes: Vec<RootNode>,
}

#[derive(Default)]
pub(crate) struct SharedNodeGraph {
    pub(crate) nodes: Vec<Node>,
    pub(crate) nodes_len: usize,
    pub(crate) first_layer: NodeChildrenFlow,
}

impl SharedNodeGraph {
    pub fn nodes(&self) -> &[Node] {
        &self.nodes[..self.nodes_len]
    }

    pub fn nodes_mut(&mut self) -> &mut [Node] {
        &mut self.nodes[..self.nodes_len]
    }

    pub fn update_nodes_len(&mut self) {
        self.nodes_len = self.nodes.len();
    }
}

// SAFETY: UnsafeCell does not implement Sync only because it was said so
unsafe impl Sync for NodeGraph
where
    SharedNodeGraph: Sync,
    bool: Sync,
    Vec<RootNode>: Sync,
{
}

impl Default for NodeGraph {
    fn default() -> Self {
        let mut node_graph = Self {
            shared: Default::default(),
            changed: true,
            root_nodes: vec![RootNode {
                bytes: vec![],
                changed: false,
                updated: false,
                policy: RootNodePolicy::Exclude(Default::default()),
            }],
        };

        node_graph.update_root_nodes(&mut HashMap::default());

        node_graph
    }
}

impl NodeGraph {
    pub(crate) fn reserve_node_id(&mut self, kind: NodeKind) -> NodeId {
        let shared = self.shared.get_mut();
        let id = shared.nodes.len();
        shared.nodes.push(Node {
            kind,
            execute: None,
            flow: NodeFlow::Stop,
            parents: Default::default(),
        });
        NodeId(id)
    }

    pub fn has_changed(&self) -> bool {
        self.changed
    }

    pub(crate) fn shared(&self) -> &SharedNodeGraph {
        // SAFETY: we are returning an immutable reference
        unsafe { &*self.shared.get() }
    }

    pub(crate) fn get_node(&self, id: NodeId) -> Option<&Node> {
        self.shared().nodes.get(id.0)
    }

    pub(crate) fn get_mut_node(&mut self, id: NodeId) -> Option<&mut Node> {
        // SAFETY: We have a mutable reference to the NodeGraph that means that there is
        // no other references from it
        unsafe { self.get_mut_node_unsafe(id) }
    }

    /// # Safety
    /// - There may not be a mutable reference to the same node
    pub(crate) unsafe fn get_mut_node_unsafe(&self, id: NodeId) -> Option<&mut Node> {
        (&mut *self.shared.get()).nodes.get_mut(id.0)
    }

    pub(crate) fn get_mut_children_flow(&mut self, id: NodeId) -> Option<&mut NodeChildrenFlow> {
        // SAFETY: We have a mutable reference to the NodeGraph that means that there is
        // no other references from it
        unsafe { self.get_mut_children_flow_unsafe(id) }
    }

    /// # Safety
    /// - There may not be a mutable reference to the same NodeChildrenFlow
    pub(crate) unsafe fn get_mut_children_flow_unsafe(
        &self,
        id: NodeId,
    ) -> Option<&mut NodeChildrenFlow> {
        let shared = &mut *self.shared.get();
        shared
            .nodes
            .get_mut(id.0)
            .map(|v| match v.flow {
                NodeFlow::Children(ref mut children_flow) => Some(children_flow.as_mut()),
                _ => None,
            })
            .unwrap_or(Some(&mut shared.first_layer))
    }

    pub(crate) fn update_root_nodes(&mut self, node2id: &mut FxHashMap<NodeId, i32>) {
        let mut root_nodes = std::mem::replace(&mut self.root_nodes, vec![]);

        for root_node in root_nodes.iter_mut() {
            root_node.updated = false;

            if self.changed || root_node.changed {
                root_node.changed = false;
                node2id.clear();
                // Shouldn't happen, so we are not returning root nodes back if error happens
                root_node.write(self, node2id).unwrap();
            }
        }

        self.changed = false;

        self.root_nodes = root_nodes;
    }

    pub(crate) fn get_root_node(&self, id: RootNodeId) -> Option<&RootNode> {
        self.root_nodes.get(id.0)
    }
}

pub struct RootNode {
    pub(crate) policy: RootNodePolicy,
    changed: bool,
    bytes: Vec<u8>,
    updated: bool,
}

impl RootNode {
    pub(crate) fn write(
        &mut self,
        graph: &NodeGraph,
        node2id: &mut FxHashMap<NodeId, i32>,
    ) -> anyhow::Result<()> {
        let mut nodes = vec![];

        self.updated = true;

        self.write_single(NodeId::ROOT, graph, node2id, &mut nodes);

        self.bytes.clear();
        pkt::CommandTreeS2c {
            commands: nodes,
            root_index: VarInt(
                *node2id
                    .get(&NodeId::ROOT)
                    .expect("There is no root entity in entity2id map"),
            ),
        }
        .encode(&mut self.bytes)
    }

    fn write_single<'a>(
        &mut self,
        node: NodeId,
        graph: &'a NodeGraph,
        node2id: &mut FxHashMap<NodeId, i32>,
        nodes: &mut Vec<pkt::Node<'a>>,
    ) -> i32 {

        if let Some(id) = node2id.get(&node) {
            return *id;
        }

        let id = nodes.len() as i32;

        node2id.insert(node, id);
        
        let node = graph.get_node(node);

        // If parser can not 'immitate' itself as brigadier's one then we say that it is
        // a greedy phrase. All children and redirects can be omitted in that
        // case. Valence will handle executions and suggestion's requests correctly
        // anyway.
        let mut children_redirect_skip = false;

        nodes.push(pkt::Node {
            children: vec![],
            data: match node {
                Some(node) => match node.kind {
                    NodeKind::Argument {
                        ref name,
                        ref parse,
                    } => match parse.obj_brigadier() {
                        Some(parser) => pkt::NodeData::Argument {
                            name: Cow::Borrowed(name.as_str()),
                            parser,
                            suggestion: parse.obj_brigadier_suggestions(),
                        },
                        None => {
                            children_redirect_skip = true;
                            // What to do with the name?
                            pkt::NodeData::Argument {
                                name: Cow::Borrowed(name.as_str()),
                                parser: pkt::Parser::String(pkt::StringArg::GreedyPhrase),
                                suggestion: Some(NodeSuggestion::AskServer),
                            }
                        }
                    },
                    NodeKind::Literal { ref name } => pkt::NodeData::Literal {
                        name: Cow::Borrowed(name.as_str()),
                    },
                },
                // Root
                None => pkt::NodeData::Root,
            },
            executable: node.and_then(|v| v.execute.as_ref()).is_some(),
            redirect_node: None,
        });

        if !children_redirect_skip {
            match node {
                Some(node) => match node.flow {
                    NodeFlow::Children(ref children_flow) => {
                        let children = children_flow
                            .children
                            .iter()
                            .filter_map(|v| {
                                self.policy
                                    .check(*v)
                                    .then(|| VarInt(self.write_single(*v, graph, node2id, nodes)))
                            })
                            .collect();

                        nodes[id as usize].children = children;
                    }
                    NodeFlow::Redirect(redirect) => {
                        if self.policy.check(redirect) {
                            let redirect = self.write_single(redirect, graph, node2id, nodes);
                            nodes[id as usize].redirect_node = Some(VarInt(redirect));
                        }
                    }
                    NodeFlow::Stop => {}
                },
                None => {
                    let children = graph
                        .shared()
                        .first_layer
                        .children
                        .iter()
                        .filter_map(|v| {
                            self.policy
                                .check(*v)
                                .then(|| VarInt(self.write_single(*v, graph, node2id, nodes)))
                        })
                        .collect();

                    nodes[id as usize].children = children;
                }
            }
        }

        id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RootNodePolicy {
    Exclude(FxHashSet<NodeId>),
    Include(FxHashSet<NodeId>),
}

impl RootNodePolicy {
    pub fn check(&self, node: NodeId) -> bool {
        match self {
            Self::Exclude(set) => !set.contains(&node),
            Self::Include(set) => set.contains(&node),
        }
    }
}

pub(crate) struct VecRootNodePolicy(Vec<u8>);

impl VecRootNodePolicy {
    pub(crate) fn add_node(&mut self, nodes_count: usize, flag: bool) {
        if nodes_count % 8 == 0 {
            self.0.push(if flag { 1 } else { 0 })
        } else {
            *self.0.get_mut(nodes_count / 8).unwrap() |= 1 << (nodes_count % 8);
        }
    }

    pub(crate) fn set_node(&mut self, index: usize, flag: bool) {
        *self.0.get_mut(index / 8).unwrap() |= 1 << (index % 8);
    }

    pub(crate) fn get_node(&self, index: usize) -> Option<bool> {
        self.0.get(index / 8).map(|v| (v & (1 << (index % 8))) == 0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RootNodeId(usize);

impl RootNodeId {
    pub const SUPER: Self = Self(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

impl NodeId {
    pub const ROOT: Self = Self(usize::MAX);
}

pub struct Node {
    pub(crate) kind: NodeKind,
    pub(crate) flow: NodeFlow,
    pub(crate) parents: SmallVec<[NodeId; 2]>,
    pub(crate) execute: Option<Box<dyn System<In = CommandArguments, Out = ()>>>,
}

impl Node {
    pub(crate) fn remove_parent(&mut self, node_id: NodeId) {
        if let Some((index, _)) = self
            .parents
            .iter()
            .enumerate()
            .find(|(_, v)| **v == node_id)
        {
            self.parents.swap_remove(index);
        }
    }
}

#[derive(Debug)]
pub(crate) enum NodeFlow {
    Children(Box<NodeChildrenFlow>),
    Redirect(NodeId),
    Stop,
}

#[derive(Debug, Default)]
pub(crate) struct NodeChildrenFlow {
    pub(crate) children: FxHashSet<NodeId>,
    pub(crate) literals: HashMap<String, NodeId>,
    pub(crate) parsers: Vec<NodeId>,
}

impl NodeChildrenFlow {
    /// Adds a new node to the children vec with respecting literals,
    /// parsers and node's parents. If this node is already added does nothing #
    pub(crate) fn add(&mut self, self_id: NodeId, node: &mut Node, node_id: NodeId) {
        if self.children.insert(node_id) {
            node.parents.push(self_id);
            match node.kind {
                NodeKind::Argument { .. } => {
                    self.parsers.push(node_id);
                }
                NodeKind::Literal { ref name } => {
                    self.literals.insert(name.clone(), node_id);
                }
            }
        }
    }

    pub(crate) fn remove(&mut self, self_id: NodeId, node: &mut Node, node_id: NodeId) {
        if self.children.remove(&node_id) {
            match node.kind {
                NodeKind::Argument { .. } => {
                    if let Some((index, _)) = self
                        .parsers
                        .iter()
                        .enumerate()
                        .find(|(_, v)| **v == node_id)
                    {
                        self.parsers.swap_remove(index);
                    }
                }
                NodeKind::Literal { ref name } => {
                    self.literals.remove(name.as_str());
                }
            }

            node.remove_parent(self_id);
        }
    }
}

pub(crate) enum NodeKind {
    Argument {
        name: String,
        parse: Box<dyn ParseObject>,
    },
    Literal {
        name: String,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum NodeSuggestion {
    AskServer,
    AllRecipes,
    AvailableSounds,
    AvailableBiomes,
    SummonableEntities,
}

/// Node of a **bevy**'s Entity not minecraft's. Block will inherit node of
/// their instances and entity may have this component on them. If there is no
/// component present then [`PrimaryNodeRoot`] will be chosen.
#[derive(Component)]
pub struct EntityNode(pub RootNodeId);

pub fn update_root_nodes(
    mut graph: ResMut<NodeGraphInWorld>,
    mut node2id: Local<FxHashMap<NodeId, i32>>,
) {
    let graph = graph.get_mut();
    graph.update_root_nodes(&mut node2id);
}

#[derive(WorldQuery)]
pub struct EntityNodeQuery(Option<Ref<'static, EntityNode>>);

impl EntityNodeQueryItem<'_> {
    pub fn get(&self) -> RootNodeId {
        self.0.as_ref().map(|v| v.0).unwrap_or(RootNodeId::SUPER)
    }

    pub fn is_changed(&self) -> bool {
        self.0.as_ref().map(|v| v.is_changed()).unwrap_or(false)
    }
}

pub fn send_nodes_to_clients(
    graph: Res<NodeGraphInWorld>,
    mut param_set: ParamSet<(
        Query<(&mut Client, EntityNodeQuery), Or<(Changed<EntityNode>, Added<Client>)>>,
        Query<(&mut Client, EntityNodeQuery)>,
    )>,
) {
    let graph = graph.get();

    // If graph has changed then any of root nodes could be changed
    if graph.has_changed() {
        for (mut client, entity_node) in param_set.p1().iter_mut() {
            let root = &graph.get_root_node(entity_node.get()).unwrap();
            if client.is_added() || entity_node.is_changed() || root.updated {
                client.write_packet(&RawCommandTreeS2c(&root.bytes));
            }
        }
    } else {
        for (mut client, entity_node) in param_set.p0().iter_mut() {
            let root = &graph.get_root_node(entity_node.get()).unwrap();
            client.write_packet(&RawCommandTreeS2c(&root.bytes));
        }
    }
}
