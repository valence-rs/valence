use std::borrow::Cow;
use std::sync::{Arc, Weak};

use bevy_ecs::prelude::{Entity, Event, EventReader, EventWriter};
use bevy_ecs::query::{QueryState, With};
use bevy_ecs::system::{Commands, Local, ParamSet, Query, Res, Resource};
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
use crate::compile::{CommandCompiler, CommandCompilerPurpose};
use crate::exec::WorldUnsafeParam;
use crate::nodes::{EntityNode, NodeChildrenFlow, NodeExclude, NodeParser, PrimaryNodeRoot};
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
    root: Query<Option<&NodeExclude>>,
    primary_root: Query<(Entity, Option<&NodeExclude>), With<PrimaryNodeRoot>>,
    entity_node: Query<Option<&EntityNode>>,
    flow: Query<Option<&NodeChildrenFlow>>,
    mut set: ParamSet<(&World, Query<&mut NodeParser>)>,
    world_unsafe: WorldUnsafeParam,
    compiler: CommandCompiler,
    queue: Res<SuggestionsQueue>,
    tokio_runtime: Res<SuggestionsTokioRuntime>,
    mut commands: Commands,
) {
    let mut parser_query = set.p1();

    for event in event.iter() {
        if !event.is_command() {
            continue;
        }

        let command = Arc::from(event.text.clone());

        let mut arc_reader = ArcStrReader::new_command(command);

        let (node_entity, node_exclude) = match event.executor.node_entity(&entity_node) {
            Some(entity) => (entity, root.get(entity).unwrap()),
            None => primary_root.single(),
        };

        let mut entity = node_entity;

        let mut reader = arc_reader.reader();

        let _ = compiler.node(
            node_entity,
            node_exclude.map(|v| &v.0),
            &mut reader,
            &mut CommandCompilerPurpose::Suggestions {
                entity: &mut entity,
            },
            node_entity,
        );

        let cursor = reader.cursor();

        // SAFETY: cursor from a reader of this Arc
        unsafe { arc_reader.set_cursor(cursor) };

        if let Ok(Some(flow)) = flow.get(entity) {
            let begin = arc_reader.cursor();

            let mut reader = arc_reader.reader();
            let literal = reader.read_unquoted_str();
            let mut current_span = StrSpan::new(begin, reader.cursor());
            let mut current_suggestions: Vec<_> = flow
                .literal
                .keys()
                .filter(|v| v.starts_with(literal))
                .map(|v| Suggestion {
                    message: v.clone(),
                    tooltip: None,
                })
                .collect();

            let mut tasks = vec![];

            for parser in flow.parsers.iter().cloned() {
                let mut reader = arc_reader.reader();
                let mut parser = parser_query.get_mut(parser).unwrap();
                let parser = parser.0.as_mut().unwrap();

                let (_, suggestions) = parser.obj_skip(&mut reader);
                tasks.push(parser.obj_suggestions(
                    suggestions,
                    arc_reader.clone(),
                    event.executor,
                    // SAFETY: &World is reserved for this system in set param and the only mutable
                    // access we have is NodeParser, which is closed for public. Anyway if there is
                    // a function that is public and using NodeParser then the system state is
                    // never will be used because it is only used in this system, so there will be
                    // no conflicts.
                    unsafe { world_unsafe.0.world() },
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

    // parsers_apply_deferred should be executed in apply_deferred
    commands.add(|world: &mut World| parsers_apply_deferred(world));
}

#[derive(Resource)]
struct ParsersApplyDeferred(
    Option<
        Box<(
            Vec<(Entity, Option<Box<dyn ParseObject>>)>,
            QueryState<(Entity, &'static mut NodeParser)>,
        )>,
    >,
);

// Note:
// - We are getting a state of this system from resource ParsersApplyDeferred
//   and replacing it while we are using it (we are doing this to get safe &mut
//   World)
// - We are moving all node parsers into the pool (TODO: a pool resource)
// - We are applying deferred
// - We are moving all node parsers back
// - We are updating query state
// - We are returning state into the resource
/// Applying deferred for each NodeParser, like [`bevy_ecs::system::Commands`]
pub fn parsers_apply_deferred(world: &mut World) {
    let pool = match world.get_resource_mut::<ParsersApplyDeferred>() {
        Some(resource) => resource,
        None => {
            let state = world.query();
            world.insert_resource(ParsersApplyDeferred(Some(Box::new((vec![], state)))));
            world.resource_mut()
        }
    }
    .into_inner();

    let mut pool = std::mem::replace(&mut pool.0, None).unwrap();

    for (entity, mut node_parser) in pool.1.iter_mut(world) {
        pool.0
            .push((entity, std::mem::replace(&mut node_parser.0, None)));
    }

    for (_, node_parser) in &mut pool.0 {
        node_parser.as_mut().unwrap().obj_apply_deferred(world);
    }

    for (entity, node_parser) in pool.0.drain(..) {
        world.entity_mut(entity).insert(NodeParser(node_parser));
    }

    pool.1.update_archetypes(world);

    world.insert_resource(ParsersApplyDeferred(Some(pool)));
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
