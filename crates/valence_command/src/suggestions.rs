use std::borrow::Cow;
use std::sync::{Arc, Weak};

use bevy_ecs::prelude::{DetectChangesMut, Entity, Event, EventReader, EventWriter};
use bevy_ecs::query::{QueryState, With};
use bevy_ecs::system::{Commands, Local, ParamSet, Query, Res, ResMut, Resource};
use bevy_ecs::world::World;
use parking_lot::Mutex;
use tokio::runtime::{Handle, Runtime};
use valence_client::event_loop::PacketEvent;
use valence_client::Client;
use valence_core::__private::VarInt;
use valence_core::protocol::encode::WritePacket;
use valence_core::protocol::packet::chat::RequestCommandCompletionsC2s;
use valence_core::protocol::{Decode, Encode};
use valence_core::text::Text;

use crate::command::CommandExecutorBase;
use crate::compile::CommandCompilerPurpose;
use crate::nodes::{EntityNodeQuery, NodeFlow, NodeGraphInWorld, NodeId, NodeKind, RootNodeId};
use crate::parse::ParseObject;
use crate::pkt;
use crate::reader::{ArcStrReader, StrLocated, StrSpan};

#[derive(Encode, Decode, Clone, Debug)]
pub struct Suggestion<'a> {
    pub message: Cow<'a, str>,
    pub tooltip: Option<Text>,
}

impl<'a> Suggestion<'a> {
    pub const fn new_str(str: &'a str) -> Self {
        Self {
            message: Cow::Borrowed(str),
            tooltip: None,
        }
    }
}

pub const CONSOLE_EVENT_ID: u32 = 0;

#[derive(Event, Debug)]
pub struct SuggestionsAnswerEvent {
    pub suggestions: StrLocated<Cow<'static, [Suggestion<'static>]>>,
    pub id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SuggestionsTransaction {
    Client { client: Entity, id: i32 },
    Event { id: u32 },
}

#[derive(Event, Clone, Debug, PartialEq)]
pub struct SuggestionsRequestEvent {
    pub transaction: SuggestionsTransaction,
    pub executor: CommandExecutorBase,
    pub text: String,
}

impl SuggestionsRequestEvent {
    pub fn is_command(&self) -> bool {
        self.text.starts_with('/')
    }
}

pub fn suggestions_request_packet(
    mut event: EventReader<PacketEvent>,
    mut request: EventWriter<SuggestionsRequestEvent>,
) {
    for packet_event in event.iter() {
        if let Some(packet) = packet_event.decode::<RequestCommandCompletionsC2s>() {
            request.send(SuggestionsRequestEvent {
                transaction: SuggestionsTransaction::Client {
                    client: packet_event.client,
                    id: packet.transaction_id.0,
                },
                executor: CommandExecutorBase::Entity {
                    entity: packet_event.client,
                },
                text: packet.text.to_string(),
            })
        }
    }
}

#[derive(Resource, Default)]
pub struct SuggestionsQueue(pub(crate) Arc<Mutex<Vec<SuggestionsCalculated>>>);

impl SuggestionsQueue {
    pub fn get(&self) -> Weak<Mutex<Vec<SuggestionsCalculated>>> {
        Arc::downgrade(&self.0)
    }
}

#[derive(Debug)]
pub struct SuggestionsCalculated {
    pub transaction: SuggestionsTransaction,
    pub suggestions: StrLocated<Cow<'static, [Suggestion<'static>]>>,
    pub command: Arc<str>,
}

#[derive(Resource)]
pub struct SuggestionsTokioRuntime {
    handle: Handle,
    _runtime: Option<Runtime>,
}

impl Default for SuggestionsTokioRuntime {
    fn default() -> Self {
        let runtime = Runtime::new().unwrap();
        Self {
            handle: runtime.handle().clone(),
            _runtime: Some(runtime),
        }
    }
}

impl SuggestionsTokioRuntime {
    pub fn with_handle(handle: Handle) -> Self {
        Self {
            handle,
            _runtime: None,
        }
    }
}

pub fn suggestions_spawn_tasks(
    mut event: EventReader<SuggestionsRequestEvent>,
    entity_node: Query<EntityNodeQuery>,
    mut set: ParamSet<(&World, ResMut<NodeGraphInWorld>)>,
    queue: Res<SuggestionsQueue>,
    tokio_runtime: Res<SuggestionsTokioRuntime>,
    mut commands: Commands,
) {
    let mut graph = set.p1().take();

    for event in event.iter() {
        if !event.is_command() {
            continue;
        }

        let command = Arc::from(event.text.clone());

        let mut arc_reader = ArcStrReader::new_command(command);

        let root = graph
            .get_root_node(
                event
                    .executor
                    .node_entity()
                    .and_then(|e| entity_node.get(e).ok().map(|v| v.get()))
                    .unwrap_or(RootNodeId::SUPER),
            )
            .unwrap();

        let mut reader = arc_reader.reader();

        let mut node = NodeId::ROOT;

        let _ = graph.walk_node(
            node,
            root,
            &mut reader,
            &mut CommandCompilerPurpose::Suggestions { node: &mut node },
        );

        let cursor = reader.cursor();

        // SAFETY: cursor from a reader of this Arc
        unsafe { arc_reader.set_cursor(cursor) };

        // SAFETY: no other references
        if let Some(flow) = unsafe { graph.get_mut_children_flow_unsafe(node) } {
            let begin = arc_reader.cursor();

            let mut reader = arc_reader.reader();
            let literal = reader.read_unquoted_str();
            let mut current_span = StrSpan::new(begin, reader.cursor());
            let mut current_suggestions: Vec<_> = flow
                .literals
                .keys()
                .filter(|v| v.starts_with(literal))
                .map(|v| Suggestion {
                    message: Cow::Owned(v.clone()),
                    tooltip: None,
                })
                .collect();

            let mut tasks = vec![];

            for parser in flow.parsers.iter().cloned() {
                let mut reader = arc_reader.reader();

                // SAFETY: NodeChildrenFlow doesn't have a reference to the self node
                let node = unsafe { graph.get_mut_node_unsafe(parser).unwrap() };
                let parser = match node.kind {
                    NodeKind::Argument { ref mut parse, .. } => parse.as_mut(),
                    NodeKind::Literal { .. } => unreachable!(),
                };

                let (_, suggestions) = parser.obj_skip(&mut reader);
                tasks.push(parser.obj_suggestions(
                    suggestions,
                    arc_reader.clone(),
                    event.executor,
                    set.p0(),
                ));
            }

            let queue = queue.get();

            let transaction = event.transaction;

            let _guard = tokio_runtime.handle.enter();

            // TODO: maybe if all future are already done we must not execute async task
            tokio::spawn(async move {
                for task in tasks {
                    let suggestions = task.await;
                    if suggestions.span == current_span {
                        match suggestions.object {
                            Cow::Owned(vec) => current_suggestions.extend(vec),
                            Cow::Borrowed(slice) => current_suggestions.extend_from_slice(slice),
                        }
                    } else if suggestions.span.is_deeper(current_span) {
                        current_suggestions = suggestions.object.into_owned();
                        current_span = suggestions.span;
                    }
                }

                if let Some(queue) = queue.upgrade() {
                    queue.lock().push(SuggestionsCalculated {
                        transaction,
                        suggestions: StrLocated::new(current_span, Cow::Owned(current_suggestions)),
                        command: arc_reader.str(),
                    });
                }
            });
        }
    }

    set.p1().insert(graph);

    // parsers_apply_deferred should be executed in apply_deferred
    commands.add(|world: &mut World| parsers_apply_deferred(world));
}

/// Applying deferred for each NodeParser, like [`bevy_ecs::system::Commands`]
pub fn parsers_apply_deferred(world: &mut World) {
    let mut graph = world.resource_mut::<NodeGraphInWorld>().take();

    for node in graph.shared.get_mut().nodes_mut().iter_mut() {
        if let NodeKind::Argument { ref mut parse, .. } = node.kind {
            parse.obj_apply_deferred(world);
        }
    }

    world.resource_mut::<NodeGraphInWorld>().insert(graph);
}

pub fn send_calculated_suggestions(
    queue: Res<SuggestionsQueue>,
    mut client_query: Query<&mut Client>,
    mut event: EventWriter<SuggestionsAnswerEvent>,
    mut suggestions_buf: Local<Vec<SuggestionsCalculated>>,
) {
    {
        let mut queue = queue.0.lock();
        std::mem::swap::<Vec<SuggestionsCalculated>>(queue.as_mut(), suggestions_buf.as_mut());
    }

    for suggestions in suggestions_buf.drain(..) {
        match suggestions.transaction {
            SuggestionsTransaction::Client { client, id } => {
                if let Ok(mut client) = client_query.get_mut(client) {
                    client.write_packet(&pkt::CommandSuggestionsS2c {
                        id: VarInt(id),
                        start: VarInt((suggestions.suggestions.span.begin().chars()) as i32),
                        length: VarInt(
                            (suggestions.suggestions.span.end().chars()
                                - suggestions.suggestions.span.begin().chars())
                                as i32,
                        ),
                        matches: Cow::Borrowed(&suggestions.suggestions.object),
                    })
                }
            }
            SuggestionsTransaction::Event { id } => event.send(SuggestionsAnswerEvent {
                suggestions: suggestions.suggestions,
                id,
            }),
        }
    }
}
