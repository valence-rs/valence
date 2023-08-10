use bevy_ecs::system::{Commands, IntoSystem, ResMut, System, SystemParam};
use bevy_ecs::world::World;

use crate::command::CommandArguments;
use crate::nodes::{NodeChildrenFlow, NodeFlow, NodeGraphInWorld, NodeId, NodeKind};
use crate::parse::{Parse, ParseWithData};

#[derive(SystemParam)]
pub struct NodeGraphCommands<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub graph: ResMut<'w, NodeGraphInWorld>,
}

impl<'w, 's> NodeGraphCommands<'w, 's> {
    pub fn spawn_literal_node<'a>(&'a mut self, name: String) -> NodeCommands<'w, 's, 'a> {
        let graph = self.graph.get_mut();
        let id = graph.reserve_node_id(NodeKind::Literal { name });
        self.commands.add(move |world: &mut World| {
            let mut graph = world.resource_mut::<NodeGraphInWorld>();
            let graph = graph.get_mut();
            graph.changed = true;
            graph.shared.get_mut().update_nodes_len();
        });
        NodeCommands { commands: self, id }
    }

    pub fn spawn_argument_node<'a, P: Parse>(
        &'a mut self,
        name: String,
        data: P::Data<'static>,
    ) -> NodeCommands<'w, 's, 'a> {
        let graph = self.graph.get_mut();
        let id = graph.reserve_node_id(NodeKind::Argument {
            name,
            parse: Box::new(ParseWithData::<P> { data, state: None }),
        });
        self.commands.add(move |world: &mut World| {
            let mut graph = world.resource_mut::<NodeGraphInWorld>().take();
            graph.changed = true;
            graph.shared.get_mut().update_nodes_len();
            let node = graph.get_mut_node(id).unwrap();
            if let NodeKind::Argument { ref mut parse, .. } = node.kind {
                parse.initialize(world);
            }
            world.resource_mut::<NodeGraphInWorld>().insert(graph);
        });
        NodeCommands { commands: self, id }
    }

    pub fn node<'a>(&'a mut self, id: NodeId) -> NodeCommands<'w, 's, 'a> {
        NodeCommands { commands: self, id }
    }
}

pub struct NodeCommands<'w, 's, 'a> {
    commands: &'a mut NodeGraphCommands<'w, 's>,
    pub id: NodeId,
}

impl<'w, 's, 'a> NodeCommands<'w, 's, 'a> {
    pub fn execute<Marker>(
        &mut self,
        system: impl IntoSystem<CommandArguments, (), Marker>,
    ) -> &mut Self {
        let node_id = self.id;
        let mut system = IntoSystem::into_system(system);
        self.commands.commands.add(move |world: &mut World| {
            system.initialize(world);
            let mut graph = world.resource_mut::<NodeGraphInWorld>();
            let graph = graph.get_mut();
            graph.changed = true;
            graph.get_mut_node(node_id).unwrap().execute = Some(Box::new(system));
        });
        self
    }

    pub fn set_redirect(&mut self, redirect: NodeId) -> &mut Self {
        let id = self.id;
        self.commands.commands.add(move |world: &mut World| {
            let mut graph = world.resource_mut::<NodeGraphInWorld>();
            let graph = graph.get_mut();

            graph.changed = true;

            // SAFETY: Only one mutable node reference
            let node = unsafe { graph.get_mut_node_unsafe(id).unwrap() };

            match node.flow {
                NodeFlow::Children(ref mut children) => {
                    for child in children.children.iter().cloned() {
                        // SAFETY: Node can not child of itself
                        unsafe { graph.get_mut_node_unsafe(child).unwrap() }.remove_parent(id);
                    }

                    node.flow = NodeFlow::Redirect(redirect);
                }
                NodeFlow::Redirect(ref mut previous) => {
                    // SAFETY: Node can not redirect to itself
                    unsafe { graph.get_mut_node_unsafe(*previous).unwrap() }.remove_parent(id);

                    *previous = redirect
                }
                NodeFlow::Stop => {
                    node.flow = NodeFlow::Redirect(redirect);
                }
            }

            // SAFETY: Node can not redirect to itself
            unsafe { graph.get_mut_node_unsafe(redirect).unwrap() }
                .parents
                .push(id);
        });
        self
    }

    /// Changes node's flow to the children, if it wasn't and then inserts all
    /// nodes from iterator
    pub fn add_children(
        &mut self,
        children: impl Iterator<Item = NodeId> + Sync + Send + 'static,
    ) -> &mut Self {
        let node_id = self.id;
        let children = children.filter(move |v| *v != node_id);
        self.commands.commands.add(move |world: &mut World| {
            let mut graph = world.resource_mut::<NodeGraphInWorld>();
            let graph = graph.get_mut();

            graph.changed = true;

            // SAFETY: Only one mutable node reference
            let node = unsafe { graph.get_mut_node_unsafe(node_id).unwrap() };

            match node.flow {
                NodeFlow::Children(ref mut children_flow) => {
                    for child in children {
                        children_flow.add(
                            node_id,
                            // SAFETY: Node's child can not be node itself
                            unsafe { graph.get_mut_node_unsafe(child).unwrap() },
                            child,
                        );
                    }
                }
                NodeFlow::Redirect(redirect) => {
                    // SAFETY: Node can not redirect to itself
                    unsafe { graph.get_mut_node_unsafe(redirect).unwrap() }.remove_parent(node_id);

                    let mut children_flow = NodeChildrenFlow::default();

                    for child in children {
                        children_flow.add(
                            node_id,
                            // SAFETY: Node's child can not be node itself
                            unsafe { graph.get_mut_node_unsafe(child).unwrap() },
                            child,
                        );
                    }

                    node.flow = NodeFlow::Children(Box::new(children_flow));
                }
                NodeFlow::Stop => {
                    let mut children_flow = NodeChildrenFlow::default();

                    for child in children {
                        children_flow.add(
                            node_id,
                            // SAFETY: Node's child can not be node itself
                            unsafe { graph.get_mut_node_unsafe(child).unwrap() },
                            child,
                        );
                    }

                    node.flow = NodeFlow::Children(Box::new(children_flow));
                }
            }
        });
        self
    }

    pub fn with_literal_child(
        &mut self,
        name: String,
        func: impl FnOnce(&mut NodeCommands),
    ) -> &mut Self {
        let mut child = self.commands.spawn_literal_node(name);
        let _ = func(&mut child);
        let id = child.id;
        self.add_children([id].into_iter())
    }

    pub fn with_argument_child<P: Parse>(
        &mut self,
        name: String,
        data: P::Data<'static>,
        func: impl FnOnce(&mut NodeCommands),
    ) -> &mut Self {
        let mut child = self.commands.spawn_argument_node::<P>(name, data);
        let _ = func(&mut child);
        let id = child.id;
        self.add_children([id].into_iter())
    }

    pub fn root_node_child(&mut self) -> &mut Self {
        let id = self.id;
        assert_ne!(id, NodeId::ROOT);
        self.commands.commands.add(move |world: &mut World| {
            let mut graph = world.resource_mut::<NodeGraphInWorld>();
            let graph = graph.get_mut();
            // SAFETY: Only one mutable reference
            let children_flow =
                unsafe { graph.get_mut_children_flow_unsafe(NodeId::ROOT).unwrap() };
            children_flow.add(
                NodeId::ROOT,
                // SAFETY: Id is not root id
                unsafe { graph.get_mut_node_unsafe(id).unwrap() },
                id,
            );
        });
        self
    }
}
