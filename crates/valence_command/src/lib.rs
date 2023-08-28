pub mod arg_parser;
pub mod command_graph;
pub mod command_scopes;
pub mod handler;
pub mod manager;

use std::collections::HashMap;
use bevy_ecs::event::Event;
use bevy_ecs::prelude::{Entity, Resource};
use petgraph::prelude::NodeIndex;
use serde_value::Value;
pub use command_scopes::CommandScopeRegistry;
use crate::arg_parser::ParseInput;

use crate::command_graph::{CommandGraph, CommandGraphBuilder};

pub trait Command {
    fn assemble_graph(graph: &mut CommandGraphBuilder<Self>) where Self: Sized;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Event)]
pub struct CommandExecutionEvent {
    /// the command that was executed eg. "teleport @p 0 ~ 0"
    pub command: String,
    /// usually the Client entity but it could be a command block or something
    /// (whatever the library user wants)
    pub executor: Entity,
}

#[derive(Resource, Default)]
pub struct CommandRegistry {
    pub graph: CommandGraph,
    pub parsers: HashMap<NodeIndex, fn(&mut ParseInput) -> bool>,
    pub modifiers: HashMap<NodeIndex, fn(String, &mut HashMap<&str, String>)>,
}