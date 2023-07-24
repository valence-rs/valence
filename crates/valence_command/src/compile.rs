use std::collections::HashMap;

use bevy_ecs::prelude::{DetectChanges, Entity, Event, EventReader, EventWriter};
use bevy_ecs::query::{Added, Changed, With};
use bevy_ecs::removal_detection::RemovedComponents;
use bevy_ecs::system::{Commands, Local, ParallelCommands, ParamSet, Query, SystemParam};
use bevy_ecs::world::World;
use rustc_hash::{FxHashMap, FxHashSet};
use valence_core::text::Text;
use valence_core::translation_key::COMMAND_EXPECTED_SEPARATOR;

use crate::command::{CommandExecutor, CommandExecutorBridge, RealCommandExecutor};
use crate::exec::CommandExecutionEvent;
use crate::nodes::{
    EntityNode, NodeChildrenFlow, NodeExclude, NodeFlow, NodeFlowInner, NodeName, NodeParents,
    NodeParser, NodeSuggestion, PrimaryNodeRoot,
};
use crate::parse::{ParseResult, ParseResults, ParseResultsWrite};
use crate::reader::{StrLocated, StrReader, StrSpan};

/// In future, command and write should be changed to a borrowed alternatives
/// like [`str`] and [`crate::parse::ParseResultsRead`] or make some pool for
/// them.
#[derive(Event)]
pub struct CompiledCommandExecutionEvent {
    pub(crate) results: ParseResults,
    pub(crate) path: Vec<Entity>,
    pub(crate) executor: CommandExecutor,
    pub(crate) real_executor: RealCommandExecutor,
}

#[derive(SystemParam)]
pub struct CommandCompiler<'w, 's> {
    flow: Query<'w, 's, (Option<&'static NodeFlow>, Option<&'static NodeChildrenFlow>)>,
    node: Query<'w, 's, (&'static NodeName, Option<&'static NodeParser>)>,
    parser: Query<'w, 's, &'static NodeParser>,
}

impl<'w, 's> CommandCompiler<'w, 's> {
    /// Searches next node to parse, parse it and inserts into `fill` then
    /// invokes this method with found node's entity until we found a node which
    /// ends everything.
    pub(crate) fn node<'a>(
        &self,
        node: Entity,
        exclude: Option<&FxHashSet<Entity>>,
        reader: &mut StrReader<'a>,
        fill: &mut ParseResultsWrite,
        path: &mut Vec<Entity>,
        commands: &mut Commands,
        root: Entity,
    ) -> ParseResult<()> {
        let (node_flow, node_children_flow) = self.flow.get(node).unwrap();

        match node_flow.map(|v| v.get()) {
            Some(NodeFlowInner::Children(children)) => {
                if reader.is_ended() {
                    path.push(node);
                    return Ok(());
                }

                match node_children_flow {
                    Some(node_children_flow) => self.handle_children(
                        node_children_flow,
                        exclude,
                        reader,
                        fill,
                        path,
                        commands,
                        root,
                    ),
                    None => {
                        let children_flow =
                            NodeChildrenFlow::new(children.iter().cloned(), &self.node);

                        let result = self.handle_children(
                            &children_flow,
                            exclude,
                            reader,
                            fill,
                            path,
                            commands,
                            root,
                        );
                        commands.entity(node).insert(children_flow);
                        result
                    }
                }
            }
            Some(NodeFlowInner::Redirect(redirect)) => {
                path.push(node);
                self.node(*redirect, exclude, reader, fill, path, commands, root)
            }
            Some(NodeFlowInner::RedirectRoot) => {
                path.push(node);
                self.node(root, exclude, reader, fill, path, commands, root)
            }
            Some(NodeFlowInner::Stop) | None => {
                path.push(node);
                Ok(())
            }
        }
    }

    fn handle_children<'a>(
        &self,
        children_flow: &NodeChildrenFlow,
        exclude: Option<&FxHashSet<Entity>>,
        reader: &mut StrReader<'a>,
        fill: &mut ParseResultsWrite,
        path: &mut Vec<Entity>,
        commands: &mut Commands,
        root: Entity,
    ) -> ParseResult<()> {
        let begin = reader.cursor();
        let mut end = begin;
        if !children_flow.literal.is_empty() {
            let literal = reader.read_unquoted_str();
            if let Some(node) = children_flow.literal.get(literal) {
                if exclude.map(|v| !v.contains(node)).unwrap_or(true) {
                    if !reader.skip_char_or_end(' ') {
                        return Err(StrLocated::new(
                            StrSpan::new(reader.cursor(), reader.cursor()),
                            Text::translate(COMMAND_EXPECTED_SEPARATOR, vec![]),
                        ));
                    }
                    return self.node(*node, exclude, reader, fill, path, commands, root);
                }
            }
            end = reader.cursor();
            // SAFETY: It was a cursor of this reader
            unsafe { reader.set_cursor(begin) };
        }

        let mut iter = children_flow.parsers.iter();

        let mut previous_err = None;

        while let Some(entity) = iter.next() {
            let begin = reader.cursor();
            let node_parser = self.parser.get(*entity).unwrap();
            // SAFETY: we drop suggestions after it and we will ensure that command will
            // live the same as fill vector
            let (result, suggestions) = unsafe { node_parser.0.obj_parse(reader, fill) };
            // SAFETY: we got suggestions from .obj_parse method
            unsafe {
                node_parser.0.obj_drop_suggestions(suggestions);
            }

            match result {
                Ok(()) => {
                    if !reader.skip_char_or_end(' ') {
                        return Err(StrLocated::new(
                            StrSpan::new(reader.cursor(), reader.cursor()),
                            Text::translate(COMMAND_EXPECTED_SEPARATOR, vec![]),
                        ));
                    }
                    return self.node(*entity, exclude, reader, fill, path, commands, root);
                }
                Err(e) => {
                    // SAFETY: begin cursor was a cursor of this reader
                    unsafe {
                        reader.set_cursor(begin);
                    }
                    previous_err.replace(e);
                }
            }
        }

        Err(match previous_err {
            Some(e) => e,
            None => StrLocated::new(StrSpan::new(begin, end), Text::text("Invalid")),
        })
    }
}

pub fn compile_commands(
    mut compiled_event: EventWriter<CompiledCommandExecutionEvent>,
    mut execution_event: EventReader<CommandExecutionEvent>,
    root: Query<Option<&NodeExclude>>,
    primary_root: Query<(Entity, Option<&NodeExclude>), With<PrimaryNodeRoot>>,
    entity_node: Query<Option<&EntityNode>>,
    compiler: CommandCompiler,
    mut cebridge: CommandExecutorBridge,
    mut commands: Commands,
) {
    for event in execution_event.iter() {
        let (node_entity, node_exclude) = match event.executor.node_entity(&entity_node) {
            Some(entity) => (entity, root.get(entity).unwrap()),
            None => primary_root.single(),
        };

        let mut results = ParseResults::new_empty(event.command.clone());
        let (mut reader, fill) = results.to_write();

        let mut path = vec![];

        match compiler.node(
            node_entity,
            node_exclude.map(|v| &v.0),
            &mut reader,
            fill,
            &mut path,
            &mut commands,
            node_entity,
        ) {
            Ok(_) => compiled_event.send(CompiledCommandExecutionEvent {
                results,
                path,
                executor: event.executor,
                real_executor: event.real_executor,
            }),
            Err(e) => {
                // TODO: error msg
                cebridge.send_message(&event.executor, e.object);
            }
        }
    }
}