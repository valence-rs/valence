use std::borrow::Cow;
use std::collections::HashMap;

use bevy_ecs::prelude::Entity;
use bevy_ecs::system::{In, IntoSystem, System};
use bevy_ecs::world::EntityMut;
use smallvec::smallvec;

use crate::command::CommandArguments;
use crate::nodes::{
    NodeChildrenFlow, NodeFlow, NodeFlowInner, NodeName, NodeParents, NodeParser, NodeSuggestion,
    NodeSystem, PCRelationVec,
};
use crate::parse::Parse;
use crate::suggestions::RawParseSuggestions;

/// Trait with all functions to create a command node
pub trait NodeEntityMut<'w>: Sized {
    /// Inserts name to the node if it wasn't already inserted
    fn name(&mut self, name: Cow<'static, str>) -> &mut Self;

    fn executor<S, Marker>(&mut self, executor: S) -> &mut Self
    where
        S: IntoSystem<CommandArguments, (), Marker>;

    fn executor_system(
        &mut self,
        executor: Box<dyn System<In = CommandArguments, Out = ()>>,
    ) -> &mut Self;

    fn parser<P>(&mut self, data: <P as Parse<'static>>::Data) -> &mut Self
    where
        for<'a> P: Parse<'a> + RawParseSuggestions<'a>;

    /// Inserts a suggestion component.
    /// [`None`] means that the suggestion will be based on brigadier parser
    /// default suggestions.
    fn suggestions(&mut self, suggestion: Option<NodeSuggestion>) -> &mut Self;

    fn add_children(&mut self, children: impl Iterator<Item = Entity>) -> &mut Self;

    fn with_child(&mut self, child: impl FnMut(&mut EntityMut)) -> &mut Self;

    fn redirect(&mut self, node: Entity) -> &mut Self;

    fn redirect_root(&mut self) -> &mut Self;

    fn stop_flow(&mut self) -> &mut Self;
}

impl<'w> NodeEntityMut<'w> for EntityMut<'w> {
    fn name(&mut self, name: Cow<'static, str>) -> &mut Self {
        if self.get::<NodeName>().is_none() {
            self.insert(NodeName(name));
        }
        self
    }

    fn executor<S, Marker>(&mut self, executor: S) -> &mut Self
    where
        S: IntoSystem<CommandArguments, (), Marker>,
    {
        let mut system: Box<S::System> = Box::new(IntoSystem::into_system(executor));
        self.executor_system(system)
    }

    fn executor_system(
        &mut self,
        mut executor: Box<dyn System<In = CommandArguments, Out = ()>>,
    ) -> &mut Self {
        self.world_scope(|world| {
            executor.initialize(world);
        });

        self.insert(NodeSystem { system: executor });

        self
    }

    fn parser<P>(&mut self, data: <P as Parse<'static>>::Data) -> &mut Self
    where
        for<'a> P: Parse<'a> + RawParseSuggestions<'a>,
    {
        self.insert(NodeParser::new::<P>(data))
    }

    fn suggestions(&mut self, suggestion: Option<NodeSuggestion>) -> &mut Self {
        match suggestion {
            Some(suggestion) => self.insert(suggestion),
            None => self.remove::<NodeSuggestion>(),
        }
    }

    fn add_children(&mut self, children: impl Iterator<Item = Entity>) -> &mut Self {
        if self.get::<NodeChildrenFlow>().is_none() {
            self.insert(NodeChildrenFlow {
                literal: HashMap::new(),
                parsers: vec![],
            });
        }

        if self.get::<NodeFlow>().is_none() {
            self.insert(NodeFlow(NodeFlowInner::Stop));
        }

        let entity = self.id();

        let unique_children = self.world_scope(|world| {
            let uworld = world.as_unsafe_world_cell();
            let unsafe_entity = uworld.get_entity(entity).unwrap();
            // SAFETY:
            // - we have permission for both of this components, because we have exclusive
            //   access
            // - they are not the same components, so we do not have two same mutable
            //   references
            let flow = unsafe { unsafe_entity.get_mut::<NodeFlow>() }
                .unwrap()
                .into_inner();
            let children_flow = unsafe { unsafe_entity.get_mut::<NodeChildrenFlow>() }
                .unwrap()
                .into_inner();

            let unique_children: PCRelationVec = match flow {
                NodeFlow(NodeFlowInner::Children(old_children)) => children
                    .filter(|v| !old_children.iter().any(|v1| v == v1))
                    .collect(),
                NodeFlow(_) => children.collect(),
            };

            for child in unique_children.iter().cloned() {
                let unsafe_child = uworld.get_entity(child).unwrap();

                // SAFETY:
                // - we have exclusive access
                // - we are not changing any archetype
                match unsafe { unsafe_child.get::<NodeParser>() } {
                    Some(_) => {
                        children_flow.parsers.push(child);
                    }
                    None => {
                        let name = unsafe { unsafe_child.get::<NodeName>() }.unwrap();
                        children_flow.literal.insert(name.cloned(), child);
                    }
                }
            }

            for child in unique_children.iter().cloned() {
                let mut child = world.entity_mut(child);

                match child.get_mut::<NodeParents>() {
                    Some(mut parents) => parents.0.push(entity),
                    None => {
                        child.insert(NodeParents(smallvec![entity]));
                    }
                }
            }

            unique_children
        });

        let mut flow = self.get_mut::<NodeFlow>().unwrap().into_inner();

        if let NodeFlow(NodeFlowInner::Children(children)) = flow {
            children.extend(unique_children);
        } else {
            flow.0 = NodeFlowInner::Children(unique_children);
        };

        self
    }

    fn with_child(&mut self, mut child: impl FnMut(&mut EntityMut)) -> &mut Self {
        let child = self.world_scope(|world| {
            let mut entity_mut = world.spawn_empty();
            child(&mut entity_mut);
            entity_mut.id()
        });
        self.add_children([child].into_iter())
    }

    fn redirect(&mut self, node: Entity) -> &mut Self {
        change_flow(self, NodeFlowInner::Redirect(node));
        self
    }

    fn redirect_root(&mut self) -> &mut Self {
        change_flow(self, NodeFlowInner::RedirectRoot);
        self
    }

    fn stop_flow(&mut self) -> &mut Self {
        change_flow(self, NodeFlowInner::Stop);
        self
    }
}

fn change_flow<'w>(entity: &mut EntityMut<'w>, flow_inner: NodeFlowInner) {
    let id = entity.id();
    entity.world_scope(|world| {
        let uworld = world.as_unsafe_world_cell();
        let entity = uworld.get_entity(id).unwrap();

        if let Some(flow) = unsafe { entity.get_mut::<NodeFlow>() } {
            let flow = flow.into_inner();
            if let NodeFlow(NodeFlowInner::Children(children)) = flow {
                for child in children.iter().cloned() {
                    let child = uworld.get_entity(child).unwrap();
                    if let Some(mut parents) = unsafe { child.get_mut::<NodeParents>() } {
                        if let Some((index, _)) =
                            parents.0.iter().enumerate().find(|(_, v)| id.eq(v))
                        {
                            parents.0.swap_remove(index);
                        }
                    }
                }

                if let Some(mut children_flow) = unsafe { entity.get_mut::<NodeChildrenFlow>() } {
                    children_flow.literal.clear();
                    children_flow.parsers.clear();
                }
            }

            flow.0 = flow_inner;
        } else {
            world.entity_mut(id).insert(NodeFlow(flow_inner));
        }
    });
}
