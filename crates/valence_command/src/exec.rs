use std::marker::PhantomData;
use std::ptr::NonNull;

use bevy_ecs::archetype::{Archetype, ArchetypeComponentId, ArchetypeGeneration};
use bevy_ecs::component::{ComponentId, Tick};
use bevy_ecs::prelude::{Entity, Event, EventReader, EventWriter};
use bevy_ecs::query::{Access, With, Without};
use bevy_ecs::system::{
    Commands, IntoSystem, ParamSet, Query, Res, Resource, System, SystemMeta, SystemParam,
    SystemParamFunction, SystemState,
};
use bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy_ecs::world::{FromWorld, World, WorldId};
use rustc_hash::FxHashMap;
use valence_client::event_loop::PacketEvent;
use valence_core::protocol::packet::chat::CommandExecutionC2s;

use crate::command::{CommandArguments, CommandExecutor, CommandExecutorBase, RealCommandExecutor};
use crate::compile::CompiledCommandExecutionEvent;
use crate::nodes::{
    EntityNode, InitializedNodeSystem, NodeExclude, NodeFlow, NodeName, NodeParents, NodeParser,
    NodeSystem, PrimaryNodeRoot,
};
use crate::parse::{ParseResultsRead, ParseResultsWrite};
use crate::reader::StrReader;

#[derive(Event, Debug)]
pub struct CommandExecutionEvent {
    pub executor: CommandExecutor,
    pub real_executor: RealCommandExecutor,
    pub command: String,
}

impl CommandExecutionEvent {
    pub fn reader(&self) -> StrReader {
        StrReader::from_command(self.command.as_str())
    }
}

pub fn command_execution_packet(
    mut event: EventReader<PacketEvent>,
    mut execution_event: EventWriter<CommandExecutionEvent>,
) {
    for packet_event in event.iter() {
        if let Some(packet) = packet_event.decode::<CommandExecutionC2s>() {
            execution_event.send(CommandExecutionEvent {
                executor: CommandExecutor::from(CommandExecutorBase::Entity {
                    entity: packet_event.client,
                }),
                real_executor: RealCommandExecutor::Player(packet_event.client),
                command: packet.command.to_string(),
            });
        }
    }
}

#[derive(Resource)]
pub struct NodeCommandExecutionInnerSystem {
    pub(crate) execution: Box<dyn System<Out = (), In = ()>>,
}

#[derive(Resource)]
pub struct NodeCommandExecutionInnerSystemAccess {
    cid: Access<ComponentId>,
    acid: Access<ArchetypeComponentId>,
}

pub(crate) struct NCEUnsafe<'w>(pub UnsafeWorldCell<'w>);

unsafe impl SystemParam for NCEUnsafe<'_> {
    type Item<'world, 'state> = NCEUnsafe<'world>;

    type State = ();

    fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        ()
    }

    unsafe fn get_param<'world, 'state>(
        _state: &'state mut Self::State,
        _system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        _change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        NCEUnsafe::<'world>(world)
    }
}

pub fn node_command_execution(world: &mut World) {
    fn node_execution(
        nce_unsafe: NCEUnsafe,
        mut execution_events: EventReader<CompiledCommandExecutionEvent>,
        mut node_system: Query<&mut NodeSystem>,
        access: Res<NodeCommandExecutionInnerSystemAccess>,
    ) {
        for event in execution_events.iter() {
            let mut executor = event.executor.clone();
            let real_executor = event.real_executor;
            let read = event.results.to_read();

            // SAFETY: safety is given by system, that we are calling
            let executor_ptr = NonNull::new(&mut executor as *mut CommandExecutor).unwrap();

            // SAFETY: above
            let read_static: ParseResultsRead<'static> = unsafe { std::mem::transmute(read) };

            for path in event.path.iter() {
                // TODO: handle this
                let node_system_component = node_system.get_mut(*path).unwrap().into_inner();
                let node_system = &mut node_system_component.system;

                let conflicts = access.cid.get_conflicts(node_system.component_access());

                if !conflicts.is_empty() {
                    panic!(
                        "Node system {} is conflicting with command execution system",
                        node_system.name()
                    )
                }

                node_system.update_archetype_component_access(nce_unsafe.0);

                // SAFETY: we checked for conflicts, if there are conflicts this system would
                // already panic
                unsafe {
                    node_system.run_unsafe(
                        (read_static.clone(), real_executor, executor_ptr),
                        nce_unsafe.0,
                    );
                }
            }
        }
    }

    let unsafe_world_cell = world.as_unsafe_world_cell();

    // initializing system if we didn't do that

    // SAFETY: there is not mutable reference to this resource
    let inner_system = match unsafe { unsafe_world_cell.world_mut() }
        .get_resource_mut::<NodeCommandExecutionInnerSystem>()
    {
        Some(inner_system) => inner_system.into_inner(),
        None => {
            let mut inner_system = NodeCommandExecutionInnerSystem {
                execution: Box::new(IntoSystem::into_system(node_execution)),
            };

            inner_system
                .execution
                .initialize(unsafe { unsafe_world_cell.world_mut() });

            let cid = inner_system.execution.component_access().clone();
            let acid = inner_system.execution.archetype_component_access().clone();

            // SAFETY: There is no references from this world
            unsafe { unsafe_world_cell.world_mut() }.insert_resource(inner_system);

            // SAFETY: we are not using any old resource references after this
            unsafe { unsafe_world_cell.world_mut() }
                .insert_resource(NodeCommandExecutionInnerSystemAccess { cid, acid });

            // SAFETY: all previous references are dropped
            unsafe { unsafe_world_cell.world_mut() }
                .resource_mut::<NodeCommandExecutionInnerSystem>()
                .into_inner()
        }
    };

    // launching system

    // SAFETY: There is only one mutable resource and it is not used in inner system
    // and in any others system because it is private
    unsafe {
        inner_system.execution.run_unsafe((), unsafe_world_cell);
    }

    // applying system

    // SAFETY: the same as above
    inner_system
        .execution
        .apply_deferred(unsafe { unsafe_world_cell.world_mut() });
}
