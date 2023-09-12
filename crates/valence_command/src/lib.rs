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

#[cfg(not(feature = "valence"))]
#[derive(Clone, Debug, PartialEq)]
pub enum Parser {
    Bool,
    Float { min: Option<f32>, max: Option<f32> },
    Double { min: Option<f64>, max: Option<f64> },
    Integer { min: Option<i32>, max: Option<i32> },
    Long { min: Option<i64>, max: Option<i64> },
    String(StringArg),
    Entity { single: bool, only_players: bool },
    GameProfile,
    BlockPos,
    ColumnPos,
    Vec3,
    Vec2,
    BlockState,
    BlockPredicate,
    ItemStack,
    ItemPredicate,
    Color,
    Component,
    Message,
    NbtCompoundTag,
    NbtTag,
    NbtPath,
    Objective,
    ObjectiveCriteria,
    Operation,
    Particle,
    Angle,
    Rotation,
    ScoreboardSlot,
    ScoreHolder { allow_multiple: bool },
    Swizzle,
    Team,
    ItemSlot,
    ResourceLocation,
    Function,
    EntityAnchor,
    IntRange,
    FloatRange,
    Dimension,
    GameMode,
    Time,
    ResourceOrTag { registry: String },
    ResourceOrTagKey { registry: String },
    Resource { registry: String },
    ResourceKey { registry: String },
    TemplateMirror,
    TemplateRotation,
    Uuid,
}

#[cfg(not(feature = "valence"))]
#[derive(Clone, Debug, PartialEq)]
pub enum NodeData {
    Root,
    Literal {
        name: String,
    },
    Argument {
        name: String,
        parser: Parser,
        suggestion: Option<Suggestion>,
    },
}

#[cfg(not(feature = "valence"))]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Suggestion {
    AskServer,
    AllRecipes,
    AvailableSounds,
    AvailableBiomes,
    SummonableEntities,
}

#[cfg(not(feature = "valence"))]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum StringArg {
    SingleWord,
    QuotablePhrase,
    GreedyPhrase,
}
