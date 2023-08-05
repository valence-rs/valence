use std::borrow::Cow;
use std::collections::HashMap;

use bevy_ecs::prelude::{Component, DetectChanges, Entity};
use bevy_ecs::query::{Added, Changed, Or, With};
use bevy_ecs::removal_detection::RemovedComponents;
use bevy_ecs::system::{Local, ParamSet, Query, System, SystemParam, SystemState};
use bevy_ecs::world::{Ref, World};
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use valence_client::Client;
use valence_core::__private::VarInt;
use valence_core::protocol::encode::WritePacket;
use valence_core::protocol::Encode;

use crate::command::CommandArguments;
use crate::parse::{Parse, ParseObject, ParseWithData};
use crate::pkt::{self, RawCommandTreeS2c};

pub(crate) type PCRelationVec = SmallVec<[Entity; 2]>;

#[derive(Component, Debug)]
pub struct NodeName(pub(crate) Cow<'static, str>);

impl NodeName {
    pub fn get(&self) -> &str {
        &self.0
    }

    pub fn cloned(&self) -> Cow<'static, str> {
        self.0.clone()
    }
}

#[derive(Component, Debug)]
pub struct NodeParents(pub(crate) PCRelationVec);

impl NodeParents {
    pub fn get(&self) -> &[Entity] {
        &self.0
    }
}

#[derive(Component)]
pub struct NodeFlow(pub(crate) NodeFlowInner);

impl NodeFlow {
    pub fn get(&self) -> &NodeFlowInner {
        &self.0
    }
}

pub enum NodeFlowInner {
    /// Saves arguments but doesn't execute the node if the reader is not empty
    Children(PCRelationVec),
    /// Redirects flow to another node but executes this node firstly. May not
    /// point to root node
    Redirect(Entity),
    /// The same as [`NodeFlow::Redirect`] but redirects to root node, which can
    /// vary depending on which root node is in use
    RedirectRoot,
    /// There is no children and redirection, the node will be executed
    Stop,
}

/// If [`NodeFlow`] is [`NodeFlow::Children`] then this component will contain
/// map of literals and parsers.
#[derive(Component)]
pub struct NodeChildrenFlow {
    pub(crate) literal: HashMap<Cow<'static, str>, Entity>,
    pub(crate) parsers: Vec<Entity>,
}

#[derive(Component)]
pub struct NodeParser(pub(crate) Option<Box<dyn ParseObject>>);

impl NodeParser {
    pub fn new<T>(data: <T as Parse<'static>>::Data, world: &mut World) -> Self
    where
        for<'a> T: Parse<'a>,
    {
        Self(Some(Box::new(ParseWithData::<T> {
            data,
            state: SystemState::new(world),
        })))
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub enum NodeSuggestion {
    AskServer,
    AllRecipes,
    AvailableSounds,
    AvailableBiomes,
    SummonableEntities,
}

#[derive(Component)]
pub struct NodeSystem {
    /// [`None`] means that this system was already moved to the system's pool
    pub(crate) system: Option<Box<dyn System<In = CommandArguments, Out = ()>>>,
}

#[derive(Component)]
pub struct InitializedNodeSystem;

#[derive(Component)]
pub struct NodeExclude(pub FxHashSet<Entity>);

/// Contains cached nodes data for this root.
#[derive(Component)]
pub struct NodeRoot(pub(crate) Vec<u8>);

#[derive(Component)]
pub struct PrimaryNodeRoot;

/// Node of a **bevy**'s Entity not minecraft's. Block will inherit node of
/// their instances and entity may have this component on them. If there is no
/// component present then [`PrimaryNodeRoot`] will be chosen.
#[derive(Component)]
pub struct EntityNode(pub Entity);

#[derive(SystemParam)]
pub struct RootWrite<'w, 's> {
    root_node: Query<'w, 's, &'static mut NodeRoot>,
    root_exclude: Query<'w, 's, Option<&'static NodeExclude>>,
    single_root_write: SingleRootWrite<'w, 's>,
    entity2id: Local<'s, FxHashMap<Entity, i32>>,
}

#[derive(SystemParam)]
struct SingleRootWrite<'w, 's> {
    node: Query<
        'w,
        's,
        (
            Option<&'static NodeParser>,
            Option<&'static NodeSystem>,
            Option<&'static NodeName>,
            Option<&'static NodeFlow>,
            Option<&'static NodeSuggestion>,
        ),
    >,
}

impl<'w, 's> RootWrite<'w, 's> {
    pub fn write_root(&mut self, root: Entity) -> anyhow::Result<()> {
        self.entity2id.clear();

        let node_exclude = self
            .root_exclude
            .get(root)
            .expect("Given entity does not exist");

        let mut nodes = vec![];

        self.single_root_write.write_node(
            &mut self.entity2id,
            node_exclude.map(|v| &v.0),
            &mut nodes,
            root,
            root,
        );

        let mut node_root = self
            .root_node
            .get_mut(root)
            .expect("Given entity is not a root node");

        node_root.0.clear();
        pkt::CommandTreeS2c {
            commands: nodes,
            root_index: VarInt(
                *self
                    .entity2id
                    .get(&root)
                    .expect("There is no root entity in entity2id map"),
            ),
        }
        .encode(&mut node_root.0)
    }
}

impl<'w, 's> SingleRootWrite<'w, 's> {
    fn write_node<'a>(
        &'a self,
        entity2id: &mut FxHashMap<Entity, i32>,
        exclude: Option<&FxHashSet<Entity>>,
        nodes: &mut Vec<pkt::Node<'a>>,
        root: Entity,
        node: Entity,
    ) -> i32 {
        let id = nodes.len() as i32;

        if entity2id.insert(node, id).is_some() {
            unreachable!();
        }

        let (node_parser, node_execute, node_name, node_flow, node_suggestion) =
            self.node.get(node).expect("Given entity is not a node");

        // If parser can not 'immitate' itself as brigadier's one then we say that it is
        // a greedy phrase. All children and redirects can be omitted in that
        // case. Valence will handle executions and suggestion's requests correctly
        // anyway.
        let mut children_redirect_skip = false;

        nodes.push(pkt::Node {
            children: vec![],
            data: match (node_parser, node_name) {
                (Some(node_parser), Some(node_name)) => {
                    match node_parser.0.as_ref().unwrap().obj_brigadier() {
                        Some(parser) => pkt::NodeData::Argument {
                            name: Cow::Borrowed(&node_name.0),
                            parser,
                            suggestion: node_suggestion.copied(),
                        },
                        None => {
                            children_redirect_skip = true;
                            pkt::NodeData::Argument {
                                name: Cow::Borrowed(&node_name.0),
                                parser: pkt::Parser::String(pkt::StringArg::GreedyPhrase),
                                suggestion: Some(NodeSuggestion::AskServer),
                            }
                        }
                    }
                }
                (None, Some(node_name)) => pkt::NodeData::Literal {
                    name: Cow::Borrowed(&node_name.0),
                },
                (..) if root == node => pkt::NodeData::Root,
                (..) => panic!(
                    "Node is targetting root node entity, use ::Root enum's variants instead"
                ),
            },
            executable: node_execute.is_some(),
            redirect_node: None,
        });

        if !children_redirect_skip {
            match node_flow.map(|v| v.get()) {
                Some(NodeFlowInner::Children(children)) => {
                    let children = children
                        .iter()
                        .filter(|v| !Self::is_excluded(exclude, *v))
                        .map(|v| VarInt(self.validate_node(entity2id, exclude, nodes, root, *v)))
                        .collect();
                    let node = &mut nodes[id as usize];
                    node.children = children;
                }
                Some(NodeFlowInner::Redirect(node)) => {
                    assert!(
                        !Self::is_excluded(exclude, node),
                        "Redirect of node is excluded from a tree"
                    );
                    let redirect_node = Some(VarInt(
                        self.validate_node(entity2id, exclude, nodes, root, *node),
                    ));
                    let node = &mut nodes[id as usize];
                    node.redirect_node = redirect_node;
                }
                Some(NodeFlowInner::RedirectRoot) => {
                    let redirect_node = Some(VarInt(
                        self.validate_node(entity2id, exclude, nodes, root, root),
                    ));
                    let node = &mut nodes[id as usize];
                    node.redirect_node = redirect_node;
                }
                Some(NodeFlowInner::Stop) | None => {}
            }
        }

        id
    }

    fn validate_node<'a>(
        &'a self,
        entity2id: &mut FxHashMap<Entity, i32>,
        exclude: Option<&FxHashSet<Entity>>,
        nodes: &mut Vec<pkt::Node<'a>>,
        root: Entity,
        node: Entity,
    ) -> i32 {
        entity2id
            .get(&node)
            .cloned()
            .unwrap_or_else(|| self.write_node(entity2id, exclude, nodes, root, node))
    }

    fn is_excluded(exclude: Option<&FxHashSet<Entity>>, node: &Entity) -> bool {
        exclude.map(|s| s.contains(node)).unwrap_or(true)
    }
}

pub fn update_root_nodes(
    query: Query<
        Entity,
        Or<(
            Added<NodeName>, // Added because NodeName is constant for each node
            Added<NodeSystem>, /* If the function of NodeSystem changes, we don't care because we
                              * are telling only if the node is
                              * executable */
            Changed<NodeFlow>,
            Changed<NodeParser>,
            Changed<NodeSuggestion>,
            Changed<NodeExclude>, // for root nodes
        )>,
    >,
    mut writer: RootWrite,
    parents: Query<Option<&NodeParents>>,
    mut updated_root_nodes: Local<FxHashSet<Entity>>,
) {
    fn iteration(
        writer: &mut RootWrite,
        parents_query: &Query<Option<&NodeParents>>,
        updated_root_nodes: &mut FxHashSet<Entity>,
        node: Entity,
    ) -> anyhow::Result<()> {
        match parents_query
            .get(node)
            .expect("Given entity does not exist")
        {
            Some(parents) => {
                for parent in parents.0.iter() {
                    iteration(writer, parents_query, updated_root_nodes, *parent)?;
                }
                Ok(())
            }
            None if updated_root_nodes.contains(&node) => Ok(()),
            None => {
                updated_root_nodes.insert(node);
                writer.write_root(node)
            }
        }
    }

    updated_root_nodes.clear();
    for node_entity in query.iter() {
        if let Err(err) = iteration(&mut writer, &parents, &mut updated_root_nodes, node_entity) {
            // TODO: log
            eprintln!("Failed to update nodes: {err:?}");
        }
    }
}

pub fn send_nodes_to_clients(
    mut param_set: ParamSet<(
        Query<(&mut Client, Option<&EntityNode>), Or<(Changed<EntityNode>, Added<Client>)>>,
        Query<(Entity, &mut Client, Option<Ref<EntityNode>>)>,
    )>,
    mut client_node_removed: RemovedComponents<EntityNode>,
    node_updated_query: Query<(), Changed<NodeRoot>>,
    root: Query<Ref<NodeRoot>>,
    root_primary: Query<Ref<NodeRoot>, With<PrimaryNodeRoot>>,
) {
    if node_updated_query.iter().next().is_none() {
        // if there is no updated root nodes then we don't need to find their listeners
        for (mut client, entity_node) in param_set.p0().iter_mut() {
            let node_root = match entity_node {
                Some(entity_node) => root.get(entity_node.0).unwrap(),
                None => root_primary.single(),
            };
            client.write_packet(&RawCommandTreeS2c(&node_root.0));
        }

        for entity in client_node_removed.iter() {
            if let Ok((_, mut client, entity_node)) = param_set.p1().get_mut(entity) {
                let node_root = match entity_node {
                    Some(entity_node) => root.get(entity_node.0).unwrap(),
                    None => root_primary.single(),
                };
                client.write_packet(&RawCommandTreeS2c(&node_root.0));
            }
        }
    } else {
        // otherwise we will check each client if their node has been updated
        // less faster
        for (client_entity, mut client, entity_node) in param_set.p1().iter_mut() {
            let (node_root, entity_node_changed) = match entity_node {
                Some(entity_node) => (root.get(entity_node.0).unwrap(), entity_node.is_changed()),
                None => (
                    root_primary.single(),
                    client_node_removed.iter().any(|e| e == client_entity),
                ),
            };
            if node_root.is_changed() || client.is_added() || entity_node_changed {
                client.write_packet(&RawCommandTreeS2c(&node_root.0));
            }
        }
    }
}
