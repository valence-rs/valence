pub mod graph;
pub mod handler;
pub mod manager;
mod modifier_value;
pub mod parsers;
pub mod scopes;

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use bevy_app::App;
use bevy_ecs::prelude::{Resource, SystemSet};
pub use manager::{CommandExecutionEvent, CommandProcessedEvent};
pub use modifier_value::ModifierValue;
use petgraph::prelude::NodeIndex;
pub use scopes::CommandScopeRegistry;

use crate::graph::{CommandGraph, CommandGraphBuilder};
use crate::handler::CommandHandler;
use crate::parsers::ParseInput;

#[derive(SystemSet, Clone, PartialEq, Eq, Hash, Debug)]
pub struct CommandSystemSet;

#[derive(Resource, Default)]
#[allow(clippy::type_complexity)]
pub struct CommandRegistry {
    pub graph: CommandGraph,
    pub parsers: HashMap<NodeIndex, fn(&mut ParseInput) -> bool>,
    pub modifiers: HashMap<NodeIndex, fn(String, &mut HashMap<ModifierValue, ModifierValue>)>,
    pub executables: HashSet<NodeIndex>,
}

pub trait Command {
    fn assemble_graph(graph: &mut CommandGraphBuilder<Self>)
        where
            Self: Sized;
}

pub trait CommandApp {
    fn add_command<T: Command + Send + Sync + Debug + 'static>(&mut self) -> &mut Self;
}

impl CommandApp for App {
    fn add_command<T: Command + Send + Sync + Debug + 'static>(&mut self) -> &mut Self {
        self.add_plugins(CommandHandler::<T>::from_command())
    }
}
