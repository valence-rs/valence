use bevy_ecs::prelude::{Entity, Event, EventReader, EventWriter};
use bevy_ecs::query::{Has, With};
use bevy_ecs::system::{Query, SystemParam};
use rustc_hash::FxHashSet;
use valence_core::text::Text;
use valence_core::translation_key::COMMAND_EXPECTED_SEPARATOR;

use crate::command::{CommandExecutor, CommandExecutorBridge, RealCommandExecutor};
use crate::exec::CommandExecutionEvent;
use crate::nodes::{
    EntityNode, NodeChildrenFlow, NodeExclude, NodeFlow, NodeFlowInner, NodeParser, NodeSystem,
    PrimaryNodeRoot,
};
use crate::parse::{ParseObject, ParseResult, ParseResults, ParseResultsWrite};
use crate::reader::{StrLocated, StrReader, StrSpan};

/// In future, command and write should be changed to a borrowed alternatives
/// like [`str`] and [`crate::parse::ParseResultsRead`] or make some pool for
/// them.
#[derive(Event)]
pub struct CompiledCommandExecutionEvent {
    pub compiled: CompiledCommand,
    pub executor: CommandExecutor,
    pub real_executor: RealCommandExecutor,
}

pub struct CompiledCommand {
    pub(crate) results: ParseResults,
    pub(crate) path: Vec<Entity>,
}

#[derive(SystemParam)]
pub struct CommandCompiler<'w, 's> {
    flow: Query<
        'w,
        's,
        (
            Option<&'static NodeFlow>,
            Option<&'static NodeChildrenFlow>,
            Has<NodeSystem>,
        ),
    >,
    parser: Query<'w, 's, &'static NodeParser>,
}

pub(crate) enum CommandCompilerPurpose<'a> {
    Execution {
        fill: &'a mut ParseResultsWrite,
        path: &'a mut Vec<Entity>,
    },
    Suggestions {
        entity: &'a mut Entity,
    },
}

impl<'a> CommandCompilerPurpose<'a> {
    pub(crate) fn last_entity(&mut self, last_entity: Entity) {
        if let Self::Suggestions { entity } = self {
            **entity = last_entity;
        }
    }

    pub(crate) fn parse(
        &mut self,
        reader: &mut StrReader,
        parser: &dyn ParseObject,
    ) -> ParseResult<()> {
        match self {
            Self::Execution { fill, .. } => parser.obj_parse(reader, fill),
            Self::Suggestions { .. } => parser.obj_skip(reader),
        }
        .0
    }

    pub(crate) fn path(
        &mut self,
        entity: Entity,
        reader: &mut StrReader,
        executable: bool,
    ) -> ParseResult<()> {
        if executable {
            if let Self::Execution { path, .. } = self {
                path.push(entity);
            }
            Ok(())
        } else {
            // TODO: translated error
            Err(StrLocated::new(
                StrSpan::new(reader.cursor(), reader.cursor()),
                Text::text("End is not executable"),
            ))
        }
    }
}

impl<'w, 's> CommandCompiler<'w, 's> {
    pub fn compile(
        &self,
        root: Entity,
        exclude: Option<&FxHashSet<Entity>>,
        command: String,
    ) -> ParseResult<CompiledCommand> {
        let mut results = ParseResults::new_empty(command);
        let (mut reader, fill) = results.to_write();
        let mut path = vec![];
        self.node(
            root,
            exclude,
            &mut reader,
            &mut CommandCompilerPurpose::Execution {
                fill,
                path: &mut path,
            },
            root,
        )?;
        Ok(CompiledCommand { results, path })
    }

    /// Searches next node to parse, parses it and inserts into `fill` then
    /// invokes this method with found node's entity until we find a node which
    /// ends everything.
    pub(crate) fn node<'a>(
        &self,
        node: Entity,
        exclude: Option<&FxHashSet<Entity>>,
        reader: &mut StrReader<'a>,
        purpose: &mut CommandCompilerPurpose,
        root: Entity,
    ) -> ParseResult<()> {
        let (node_flow, node_children_flow, executable) = self.flow.get(node).unwrap();

        match node_flow.map(|v| v.get()) {
            Some(NodeFlowInner::Children(_)) => {
                if reader.is_ended() {
                    return purpose.path(node, reader, executable);
                }

                self.handle_children(node_children_flow.unwrap(), exclude, reader, purpose, root)
            }
            Some(NodeFlowInner::Redirect(redirect)) => {
                purpose.path(node, reader, executable)?;
                purpose.last_entity(*redirect);
                self.node(*redirect, exclude, reader, purpose, root)
            }
            Some(NodeFlowInner::RedirectRoot) => {
                purpose.path(node, reader, executable)?;
                purpose.last_entity(root);
                self.node(root, exclude, reader, purpose, root)
            }
            Some(NodeFlowInner::Stop) | None => {
                purpose.path(node, reader, executable)?;
                Ok(())
            }
        }
    }

    fn handle_children<'a>(
        &self,
        children_flow: &NodeChildrenFlow,
        exclude: Option<&FxHashSet<Entity>>,
        reader: &mut StrReader<'a>,
        purpose: &mut CommandCompilerPurpose,
        root: Entity,
    ) -> ParseResult<()> {
        let begin = reader.cursor();
        let mut end = begin;
        if !children_flow.literal.is_empty() {
            let literal = reader.read_unquoted_str();
            if let Some(node) = children_flow.literal.get(literal) {
                if exclude.map(|v| !v.contains(node)).unwrap_or(true) {
                    if reader.is_ended() {
                        return Ok(());
                    }

                    if reader.skip_char(' ') {
                        purpose.last_entity(*node);
                    } else {
                        return Err(StrLocated::new(
                            StrSpan::new(reader.cursor(), reader.cursor()),
                            Text::translate(COMMAND_EXPECTED_SEPARATOR, vec![]),
                        ));
                    }
                    return self.node(*node, exclude, reader, purpose, root);
                }
            }
            end = reader.cursor();
            // SAFETY: It was a cursor of this reader
            unsafe { reader.set_cursor(begin) };
        }

        let mut previous_err: Option<StrLocated<Text>> = None;

        for entity in children_flow.parsers.iter().cloned() {
            let begin = reader.cursor();
            let node_parser = self.parser.get(entity).unwrap();
            let result = purpose.parse(reader, node_parser.0.as_ref().unwrap().as_ref());

            match result {
                Ok(()) => {
                    if reader.is_ended() {
                        return Ok(());
                    }

                    if reader.skip_char(' ') {
                        purpose.last_entity(entity);
                    } else {
                        return Err(StrLocated::new(
                            StrSpan::new(reader.cursor(), reader.cursor()),
                            Text::translate(COMMAND_EXPECTED_SEPARATOR, vec![]),
                        ));
                    }
                    return self.node(entity, exclude, reader, purpose, root);
                }
                Err(e) => {
                    // SAFETY: begin cursor was a cursor of this reader
                    unsafe { reader.set_cursor(begin) };
                    match previous_err {
                        Some(ref mut previous_err) => {
                            let prev_span = previous_err.span;
                            let cur_span = e.span;

                            // Checking which error message is deeper
                            if cur_span.is_deeper(prev_span) {
                                *previous_err = e;
                            }
                        }
                        None => {
                            previous_err.replace(e);
                        }
                    }
                }
            }
        }

        Err(match previous_err {
            Some(e) => e,
            // TODO: translated error
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
) {
    for event in execution_event.iter() {
        let (node_entity, node_exclude) = match event.executor.node_entity(&entity_node) {
            Some(entity) => (entity, root.get(entity).unwrap()),
            None => primary_root.single(),
        };

        match compiler.compile(
            node_entity,
            node_exclude.map(|v| &v.0),
            event.command.clone(),
        ) {
            Ok(compiled) => compiled_event.send(CompiledCommandExecutionEvent {
                compiled,
                executor: event.executor,
                real_executor: event.real_executor,
            }),
            Err(e) => {
                // TODO: error msg
                cebridge.send_message(event.real_executor, e.object);
            }
        }
    }
}
