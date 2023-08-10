use bevy_ecs::prelude::{Entity, Event, EventReader, EventWriter};
use bevy_ecs::query::{Has, With};
use bevy_ecs::system::{Query, Res, SystemParam};
use rustc_hash::FxHashSet;
use valence_core::text::Text;
use valence_core::translation_key::COMMAND_EXPECTED_SEPARATOR;

use crate::command::{CommandExecutor, CommandExecutorBridge, RealCommandExecutor};
use crate::exec::CommandExecutionEvent;
use crate::nodes::{
    EntityNode, EntityNodeQuery, NodeChildrenFlow, NodeFlow, NodeGraph, NodeGraphInWorld, NodeId,
    NodeKind, RootNode, RootNodeId,
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
    pub(crate) path: Vec<NodeId>,
}

pub(crate) enum CommandCompilerPurpose<'a> {
    Execution {
        fill: &'a mut ParseResultsWrite,
        path: &'a mut Vec<NodeId>,
    },
    Suggestions {
        node: &'a mut NodeId,
    },
}

impl<'a> CommandCompilerPurpose<'a> {
    pub(crate) fn last_node(&mut self, last_node: NodeId) {
        if let Self::Suggestions { node: entity } = self {
            **entity = last_node;
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

    pub(crate) fn add_path(&mut self, node: NodeId) {
        if let Self::Execution { path, .. } = self {
            path.push(node);
        }
    }

    pub(crate) fn path(
        &mut self,
        node: NodeId,
        reader: &mut StrReader,
        executable: bool,
    ) -> ParseResult<()> {
        if executable {
            self.add_path(node);
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

impl NodeGraph {
    pub fn compile_command(
        &self,
        root: &RootNode,
        command: String,
    ) -> ParseResult<CompiledCommand> {
        let mut results = ParseResults::new_empty(command);
        let (mut reader, fill) = results.to_write();
        let mut path = vec![];
        self.walk_node(
            NodeId::ROOT,
            root,
            &mut reader,
            &mut CommandCompilerPurpose::Execution {
                fill,
                path: &mut path,
            },
        )?;
        Ok(CompiledCommand { results, path })
    }

    pub(crate) fn walk_node<'a>(
        &self,
        node_id: NodeId,
        root: &RootNode,
        reader: &mut StrReader<'a>,
        purpose: &mut CommandCompilerPurpose,
    ) -> ParseResult<()> {
        match self.get_node(node_id) {
            Some(node) => match node.flow {
                NodeFlow::Children(ref children_flow) => {
                    if reader.is_ended() {
                        purpose.path(node_id, reader, node.execute.is_some())
                    } else {
                        self.walk_children(children_flow.as_ref(), root, reader, purpose)
                    }
                }
                NodeFlow::Redirect(redirect) => {
                    if node.execute.is_some() {
                        purpose.add_path(redirect);
                    }
                    if root.policy.check(redirect) {
                        purpose.last_node(redirect);
                        self.walk_node(redirect, root, reader, purpose)
                    } else {
                        purpose.last_node(node_id);
                        Ok(())
                    }
                }
                NodeFlow::Stop => {
                    purpose.path(node_id, reader, node.execute.is_some())?;
                    purpose.last_node(node_id);
                    Ok(())
                }
            },
            None => self.walk_children(&self.shared().first_layer, root, reader, purpose),
        }
    }

    pub(crate) fn walk_children<'a>(
        &self,
        children_flow: &NodeChildrenFlow,
        root: &RootNode,
        reader: &mut StrReader<'a>,
        purpose: &mut CommandCompilerPurpose,
    ) -> ParseResult<()> {
        let begin = reader.cursor();
        let mut end = begin;

        if !children_flow.literals.is_empty() {
            let literal = reader.read_unquoted_str();
            if let Some(node) = children_flow.literals.get(literal) {
                if root.policy.check(*node) {
                    if reader.is_ended() {
                    } else if reader.skip_char(' ') {
                        purpose.last_node(*node);
                    } else {
                        return Err(StrLocated::new(
                            StrSpan::new(reader.cursor(), reader.cursor()),
                            Text::translate(COMMAND_EXPECTED_SEPARATOR, vec![]),
                        ));
                    }

                    return self.walk_node(*node, root, reader, purpose);
                }
            }
            end = reader.cursor();
            // SAFETY: It was a cursor of this reader
            unsafe { reader.set_cursor(begin) };
        }

        let mut previous_err: Option<StrLocated<Text>> = None;

        for node_id in children_flow.parsers.iter().cloned() {
            if !root.policy.check(node_id) {
                continue;
            }

            // It is impossible for root node to be here
            let node = self.get_node(node_id).unwrap();
            let result = purpose.parse(
                reader,
                match node.kind {
                    NodeKind::Argument { ref parse, .. } => parse.as_ref(),
                    _ => unreachable!(),
                },
            );

            match result {
                Ok(()) => {
                    if reader.is_ended() {
                    } else if reader.skip_char(' ') {
                        purpose.last_node(node_id);
                    } else {
                        return Err(StrLocated::new(
                            StrSpan::new(reader.cursor(), reader.cursor()),
                            Text::translate(COMMAND_EXPECTED_SEPARATOR, vec![]),
                        ));
                    }
                    return self.walk_node(node_id, root, reader, purpose);
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
    graph: Res<NodeGraphInWorld>,
    mut compiled_event: EventWriter<CompiledCommandExecutionEvent>,
    mut execution_event: EventReader<CommandExecutionEvent>,
    entity_node: Query<EntityNodeQuery>,
    mut cebridge: CommandExecutorBridge,
) {
    let graph = graph.get();
    for event in execution_event.iter() {
        let root = graph
            .get_root_node(
                event
                    .executor
                    .node_entity()
                    .and_then(|e| entity_node.get(e).ok())
                    .map(|v| v.get())
                    .unwrap_or(RootNodeId::SUPER),
            )
            .unwrap();

        match graph.compile_command(&root, event.command.clone()) {
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
