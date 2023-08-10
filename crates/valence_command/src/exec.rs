use std::ptr::NonNull;

use bevy_ecs::archetype::ArchetypeGeneration;
use bevy_ecs::component::{ComponentId, Tick};
use bevy_ecs::prelude::{Entity, Event, EventReader, EventWriter};
use bevy_ecs::query::{Access, Changed};
use bevy_ecs::removal_detection::RemovedComponents;
use bevy_ecs::system::{
    IntoSystem, Local, ParamSet, Query, Res, ResMut, Resource, System, SystemMeta, SystemParam,
};
use bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy_ecs::world::{FromWorld, World};
use rustc_hash::FxHashMap;
use valence_client::event_loop::PacketEvent;
use valence_core::protocol::packet::chat::CommandExecutionC2s;

use crate::command::{CommandArguments, CommandExecutor, CommandExecutorBase, RealCommandExecutor};
use crate::compile::CompiledCommandExecutionEvent;
use crate::nodes::NodeGraphInWorld;
use crate::parse::ParseResultsRead;
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
    pub(crate) execution: Option<Box<dyn System<Out = (), In = ()>>>,
}

#[derive(Resource)]
pub struct NodeCommandExecutionInnerSystemAccess {
    cid: Access<ComponentId>,
}

impl NodeCommandExecutionInnerSystemAccess {
    pub fn is_conflicting<In: 'static, Out: 'static>(
        &self,
        system: &dyn System<In = In, Out = Out>,
    ) -> bool {
        !self.cid.get_conflicts(system.component_access()).is_empty()
    }
}

#[doc(hidden)]
pub struct WorldUnsafeParam<'w>(pub(crate) UnsafeWorldCell<'w>);

unsafe impl SystemParam for WorldUnsafeParam<'_> {
    type Item<'world, 'state> = WorldUnsafeParam<'world>;

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
        WorldUnsafeParam::<'world>(world)
    }
}

pub fn node_command_execution(world: &mut World) {
    fn node_execution(
        mut graph: ResMut<NodeGraphInWorld>,
        nce_unsafe: WorldUnsafeParam,
        mut execution_events: EventReader<CompiledCommandExecutionEvent>,
        access: Res<NodeCommandExecutionInnerSystemAccess>,
        mut old_archetype_generation: Local<Option<ArchetypeGeneration>>,
    ) {
        let mut graph = graph.take();

        let new_archetype_generation = nce_unsafe.0.archetypes().generation();
        if Some(new_archetype_generation) != *old_archetype_generation {
            *old_archetype_generation = Some(new_archetype_generation);
            for node in graph.shared.get_mut().nodes_mut().iter_mut() {
                if let Some(ref mut system) = node.execute {
                    system.update_archetype_component_access(nce_unsafe.0);
                }
            }
        }

        for event in execution_events.iter() {
            let mut executor = event.executor.clone();
            let real_executor = event.real_executor;
            let read = event.compiled.results.to_read();

            // SAFETY: safety is given by system, that we are calling
            let executor_ptr = NonNull::new(&mut executor as *mut CommandExecutor).unwrap();

            // SAFETY: above
            let read_static: ParseResultsRead<'static> = unsafe { std::mem::transmute(read) };

            for path in event.compiled.path.iter() {
                let node = graph.get_mut_node(*path).unwrap().execute.as_mut().unwrap();

                // SAFETY: all systems are checked for conflicts
                unsafe {
                    node.run_unsafe(
                        (read_static.clone(), real_executor, executor_ptr),
                        nce_unsafe.0,
                    );
                }
            }
        }

        // We want to ensure that nothing will be used further (Also drop, which can in
        // theory use UnsafeWorldCell)
        drop(execution_events);
        drop(access);

        // SAFETY: no SystemParams, which require any world, will be used further
        let world = unsafe { nce_unsafe.0.world_mut() };

        for node in graph.shared.get_mut().nodes_mut().iter_mut() {
            if let Some(ref mut system) = node.execute {
                system.update_archetype_component_access(nce_unsafe.0);
            }
        }

        world.resource_mut::<NodeGraphInWorld>().insert(graph);
    }

    let unsafe_world_cell = world.as_unsafe_world_cell();

    // initializing system if we didn't do that

    // SAFETY: there is not mutable reference to this resource
    let inner_system = match unsafe { unsafe_world_cell.world_mut() }
        .get_resource_mut::<NodeCommandExecutionInnerSystem>()
    {
        Some(inner_system) => inner_system.into_inner(),
        None => {
            let mut inner_system = Box::new(IntoSystem::into_system(node_execution));

            inner_system.initialize(unsafe { unsafe_world_cell.world_mut() });

            let cid = inner_system.component_access().clone();
            // let acid = inner_system.archetype_component_access().clone();

            // SAFETY: There is no references from this world
            unsafe { unsafe_world_cell.world_mut() }.insert_resource(
                NodeCommandExecutionInnerSystem {
                    execution: Some(inner_system),
                },
            );

            // SAFETY: we are not using any old resource references after this
            unsafe { unsafe_world_cell.world_mut() }
                .insert_resource(NodeCommandExecutionInnerSystemAccess { cid });

            // SAFETY: all previous references are dropped
            unsafe { unsafe_world_cell.world_mut() }
                .resource_mut::<NodeCommandExecutionInnerSystem>()
                .into_inner()
        }
    };

    let mut execution_system = std::mem::replace(&mut inner_system.execution, None).unwrap();

    // SAFETY:
    // - we don't have anything from the world
    let world = unsafe { unsafe_world_cell.world_mut() };

    // launching system

    execution_system.run((), world);

    // applying system

    execution_system.apply_deferred(world);

    // we are returning our system to the resource
    world
        .resource_mut::<NodeCommandExecutionInnerSystem>()
        .execution = Some(execution_system);
}
