use std::collections::HashMap;
use std::marker::PhantomData;

use bevy_app::{App, Plugin, PostStartup};
use bevy_ecs::change_detection::ResMut;
use bevy_ecs::event::{Event, EventReader, EventWriter};
use bevy_ecs::prelude::{Entity, IntoSystemConfigs, Resource};
use petgraph::prelude::NodeIndex;
use valence_server::EventLoopPreUpdate;

use crate::graph::CommandGraphBuilder;
use crate::modifier_value::ModifierValue;
use crate::parsers::ParseInput;
use crate::{
    Command, CommandProcessedEvent, CommandRegistry, CommandScopeRegistry, CommandSystemSet,
};

impl<T> Plugin for CommandHandlerPlugin<T>
where
    T: Command + Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        app.add_event::<CommandResultEvent<T>>()
            .insert_resource(CommandResource::<T>::new())
            .add_systems(PostStartup, command_startup_system::<T>)
            .add_systems(
                EventLoopPreUpdate,
                command_event_system::<T>.after(CommandSystemSet),
            );
    }
}

pub struct CommandHandlerPlugin<T>
where
    T: Command,
{
    command: PhantomData<T>,
}

impl<T: Command> Default for CommandHandlerPlugin<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> CommandHandlerPlugin<T>
where
    T: Command,
{
    pub fn new() -> Self {
        CommandHandlerPlugin {
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

fn command_startup_system<T>(
    mut registry: ResMut<CommandRegistry>,
    mut scope_registry: ResMut<CommandScopeRegistry>,
    mut command: ResMut<CommandResource<T>>,
) where
    T: Command + Send + Sync + 'static,
{
    let mut executables = HashMap::new();
    let mut parsers = HashMap::new();
    let mut modifiers = HashMap::new();
    let graph_builder = &mut CommandGraphBuilder::new(
        &mut registry,
        &mut executables,
        &mut parsers,
        &mut modifiers,
    );
    T::assemble_graph(graph_builder);
    graph_builder.apply_scopes(&mut scope_registry);

    command.executables.extend(executables.clone());
    registry.parsers.extend(parsers);
    registry.modifiers.extend(modifiers);
    registry.executables.extend(executables.keys());
}

/// This system reads incoming command events.
fn command_event_system<T>(
    mut command_processed_events: EventReader<CommandProcessedEvent>,
    mut command_result_events: EventWriter<CommandResultEvent<T>>,
    command: ResMut<CommandResource<T>>,
) where
    T: Command + Send + Sync,
{
    for command_processed in command_processed_events.read() {
        if let Some(executable) = command.executables.get(&command_processed.node) {
            let result = executable(&mut ParseInput::new(&command_processed.command));
            command_result_events.send(CommandResultEvent {
                result,
                executor: command_processed.executor,
                modifiers: command_processed.modifiers.clone(),
            });
        }
    }
}
