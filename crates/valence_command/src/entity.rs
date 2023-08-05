use std::borrow::Cow;

use bevy_ecs::prelude::Entity;
use bevy_ecs::system::{Commands, IntoSystem};
use bevy_ecs::world::{EntityMut, World};
use rustc_hash::FxHashSet;

use crate::command::CommandArguments;
use crate::nodes::{NodeExclude, NodeRoot, NodeSuggestion, PrimaryNodeRoot};
use crate::parse::Parse;
use crate::world::NodeEntityMut;

pub trait NodeEntityCommandGet<'w, 's> {
    fn spawn_node<'c>(&'c mut self) -> NodeEntityCommands<'w, 's, 'c>;

    fn spawn_root_node<'c>(&'c mut self, primary: bool) -> NodeEntityCommands<'w, 's, 'c>;

    fn node<'c>(&'c mut self, entity: Entity) -> NodeEntityCommands<'w, 's, 'c>;
}

impl<'w, 's> NodeEntityCommandGet<'w, 's> for Commands<'w, 's> {
    fn spawn_node<'c>(&'c mut self) -> NodeEntityCommands<'w, 's, 'c> {
        let entity = self.spawn_empty().id();
        NodeEntityCommands {
            entity,
            commands: self,
        }
    }

    fn spawn_root_node<'c>(&'c mut self, primary: bool) -> NodeEntityCommands<'w, 's, 'c> {
        let mut entity = self.spawn_empty();
        entity.insert(NodeRoot(vec![]));
        entity.insert(NodeExclude(FxHashSet::default()));
        if primary {
            entity.insert(PrimaryNodeRoot);
        }
        let entity = entity.id();
        NodeEntityCommands {
            entity,
            commands: self,
        }
    }

    fn node<'c>(&'c mut self, entity: Entity) -> NodeEntityCommands<'w, 's, 'c> {
        let entity = self.entity(entity).id();
        NodeEntityCommands {
            entity,
            commands: self,
        }
    }
}

pub struct NodeEntityCommands<'w, 's, 'c> {
    entity: Entity,
    commands: &'c mut Commands<'w, 's>,
}

impl<'w, 's, 'c> NodeEntityCommands<'w, 's, 'c> {
    /// Inserts name to the node if it wasn't already inserted
    pub fn name(&mut self, name: Cow<'static, str>) -> &mut Self {
        self.add(|entity| {
            entity.name(name);
        });
        self
    }

    pub fn executor<System, Marker>(&mut self, executor: System) -> &mut Self
    where
        System: IntoSystem<CommandArguments, (), Marker> + Send,
    {
        let executor = Box::new(IntoSystem::into_system(executor));
        self.add(move |entity| {
            entity.executor_system(executor);
        });
        self
    }

    pub fn parser<P: Parse>(&mut self, data: P::Data<'static>) -> &mut Self {
        self.add(move |entity| {
            entity.parser::<P>(data);
        });
        self
    }

    /// Inserts a suggestion component.
    /// [`None`] means that the suggestion will be based on brigadier parser
    /// default suggestions.
    pub fn suggestions(&mut self, suggestion: Option<NodeSuggestion>) -> &mut Self {
        self.add(move |entity| {
            entity.suggestions(suggestion);
        });
        self
    }

    pub fn add_children(
        &mut self,
        children: impl Iterator<Item = Entity> + Send + 'static,
    ) -> &mut Self {
        self.add(move |entity| {
            entity.add_children(children);
        });
        self
    }

    pub fn with_child(&mut self, mut child: impl FnMut(&mut NodeEntityCommands)) -> &mut Self {
        let mut child_commands = self.commands.spawn_node();
        child(&mut child_commands);
        let entity = child_commands.entity;
        self.add_children([entity].into_iter())
    }

    pub fn redirect(&mut self, node: Entity) -> &mut Self {
        self.add(move |entity| {
            entity.redirect(node);
        });
        self
    }

    pub fn redirect_root(&mut self) -> &mut Self {
        self.add(move |entity| {
            entity.redirect_root();
        });
        self
    }

    pub fn stop_flow(&mut self) -> &mut Self {
        self.add(move |entity| {
            entity.stop_flow();
        });
        self
    }

    fn add(&mut self, func: impl FnOnce(&mut EntityMut) + Send + 'static) {
        let entity = self.entity;
        self.commands
            .add(move |world: &mut World| func(&mut world.entity_mut(entity)));
    }
}
