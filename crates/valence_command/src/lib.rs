pub mod arg_parser;
pub mod command_graph;
pub mod command_scopes;
pub mod handler;
pub mod manager;
mod modifier_value;

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use bevy_app::{App, Update};
use bevy_ecs::event::Event;
use bevy_ecs::prelude::{Entity, IntoSystemConfigs, Resource, SystemSet};
pub use command_scopes::CommandScopeRegistry;
pub use modifier_value::ModifierValue;
use petgraph::prelude::NodeIndex;

use crate::arg_parser::ParseInput;
use crate::command_graph::{CommandGraph, CommandGraphBuilder};
use crate::handler::CommandHandler;

pub trait Command {
    fn assemble_graph(graph: &mut CommandGraphBuilder<Self>)
    where
        Self: Sized;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Event)]
pub struct CommandExecutionEvent {
    /// the command that was executed eg. "teleport @p 0 ~ 0"
    pub command: String,
    /// usually the Client entity but it could be a command block or something
    /// (whatever the library user wants)
    pub executor: Entity,
}

/// this will only be sent if the command was successfully parsed and an
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

#[derive(SystemSet, Clone, PartialEq, Eq, Hash, Debug)]
pub enum CommandSystemSet {
    Update,
    PostUpdate,
}

#[derive(Resource, Default)]
#[allow(clippy::type_complexity)]
pub struct CommandRegistry {
    pub graph: CommandGraph,
    pub parsers: HashMap<NodeIndex, fn(&mut ParseInput) -> bool>,
    pub modifiers: HashMap<NodeIndex, fn(String, &mut HashMap<ModifierValue, ModifierValue>)>,
    pub executables: HashSet<NodeIndex>,
}

pub trait CommandApp {
    fn add_command<T: Command + Send + Sync + Debug + 'static>(&mut self) -> &mut Self;
    fn add_command_handlers<M>(&mut self, systems: impl IntoSystemConfigs<M>) -> &mut Self;
}

impl CommandApp for App {
    fn add_command<T: Command + Send + Sync + Debug + 'static>(&mut self) -> &mut Self {
        self.add_plugins(CommandHandler::<T>::from_command())
    }

    fn add_command_handlers<M>(&mut self, systems: impl IntoSystemConfigs<M>) -> &mut Self {
        self.add_systems(Update, systems.in_set(CommandSystemSet::PostUpdate))
    }
}
