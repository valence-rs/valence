use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Instant;

use bevy_app::{App, Plugin, PostStartup, Update};
use bevy_ecs::change_detection::{Res, ResMut};
use bevy_ecs::event::{Event, EventReader, EventWriter};
use bevy_ecs::prelude::{Entity, Resource};
use bevy_ecs::system::Query;

use petgraph::prelude::NodeIndex;
use petgraph::Graph;
use tracing::trace;

use crate::arg_parser::ParseInput;
use crate::command_graph::{CommandEdgeType, CommandGraphBuilder, CommandNode, NodeData};
use crate::command_scopes::CommandScopes;
use crate::{Command, CommandExecutionEvent, CommandProcessedEvent, CommandRegistry, CommandScopeRegistry};
use crate::modifier_value::ModifierValue;

pub struct CommandHandler<T>
where
    T: Command,
{
    command: PhantomData<T>,
}

impl<T> CommandHandler<T>
where
    T: Command,
{
    pub fn from_command() -> Self {
        CommandHandler {
            command: PhantomData,
        }
    }
}

#[derive(Resource)]
struct CommandResource<T: Command + Send + Sync> {
    command: PhantomData<T>,
    executables: HashMap<NodeIndex, fn(&mut ParseInput) -> T>,
}

impl<T: Command + Send + Sync> CommandResource<T> {
    pub fn new() -> Self {
        CommandResource {
            command: PhantomData,
            executables: HashMap::new(),
        }
    }
}

#[derive(Event)]
pub struct CommandResultEvent<T>
where
    T: Command,
    T: Send + Sync + 'static,
{
    pub result: T,
    pub executor: Entity,
    pub modifiers: HashMap<ModifierValue, ModifierValue>,
}

impl<T> Plugin for CommandHandler<T>
where
    T: Command + Send + Sync + Debug + 'static,
{
    fn build(&self, app: &mut App) {
        app.add_event::<CommandResultEvent<T>>()
            .insert_resource(CommandResource::<T>::new())
            .add_systems(Update, command_event_system::<T>)
            .add_systems(PostStartup, command_startup_system::<T>);
    }
}

fn command_startup_system<T>(
    mut registry: ResMut<CommandRegistry>,
    mut command: ResMut<CommandResource<T>>,
) where
    T: Command + Send + Sync + 'static,
{
    let mut executables = HashMap::new();
    let mut parsers = HashMap::new();
    let mut modifiers = HashMap::new();
    let graph_builder =
        &mut CommandGraphBuilder::new(&mut registry, &mut executables, &mut parsers, &mut modifiers);
    T::assemble_graph(graph_builder);
    command.executables.extend(executables.clone());
    registry.parsers.extend(parsers);
    registry.modifiers.extend(modifiers);
    registry.executables.extend(executables.keys());

    println!("Command graph: {}", registry.graph);
}

/// this system reads incoming command events and prints them to the console
fn command_event_system<T>(
    mut commands_executed: EventReader<CommandProcessedEvent>,
    mut events: EventWriter<CommandResultEvent<T>>,
    command: ResMut<CommandResource<T>>,
) where
    T: Command + Send + Sync + Debug,
{
    for command_event in commands_executed.iter() {
        if command.executables.contains_key(&command_event.node) {
            let timer = Instant::now();
            let result = command.executables.get(&command_event.node).unwrap()(&mut ParseInput::new(
                &command_event.command,
            ));
            events.send(CommandResultEvent { result, executor: command_event.executor, modifiers: command_event.modifiers.clone() });
            println!("Command took: {:?}", timer.elapsed());
        }
    }
}


