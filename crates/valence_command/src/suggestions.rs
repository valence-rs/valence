use std::borrow::Cow;
use std::ptr::NonNull;

use bevy_ecs::prelude::{Entity, Event, EventWriter};
use bevy_ecs::system::{Query, SystemParam};
use bevy_ecs::world::World;
use valence_client::Client;
use valence_core::__private::VarInt;
use valence_core::protocol::encode::WritePacket;
use valence_core::protocol::packet::chat::{CommandSuggestionsMatch, CommandSuggestionsS2c};
use valence_core::text::Text;

use crate::command::{CommandExecutor, RealCommandExecutor};
use crate::parse::{Parse, ParseWithData};
use crate::reader::StrLocated;

#[derive(Clone, Debug)]
pub struct Suggestion<'a> {
    pub message: Cow<'a, str>,
    pub tooltip: Option<Text>,
}

impl<'a> Suggestion<'a> {
    pub(crate) fn clone_static(&self) -> Suggestion<'static> {
        Suggestion {
            message: Cow::Owned(self.message.to_string()),
            tooltip: self.tooltip.clone(),
        }
    }
}

pub const CONSOLE_EVENT_ID: u32 = 0;

#[derive(Event, Clone, Debug)]
pub struct SuggestionAnswerEvent {
    pub suggestions: StrLocated<Vec<Suggestion<'static>>>,
    pub id: u32,
}

#[derive(SystemParam)]
pub struct SuggestionAnswerer<'w, 's> {
    client: Query<'w, 's, &'static mut Client>,
    event: EventWriter<'w, SuggestionAnswerEvent>,
}

impl<'w, 's> SuggestionAnswerer<'w, 's> {
    pub fn answer(
        &mut self,
        transaction: SuggestionsTransaction,
        suggestions: StrLocated<Cow<[Suggestion]>>,
    ) {
        match transaction {
            SuggestionsTransaction::Event { id } => {
                self.event.send(SuggestionAnswerEvent {
                    suggestions: suggestions
                        .map(|v| v.into_iter().map(|v| v.clone_static()).collect()),
                    id,
                });
            }
            SuggestionsTransaction::Client { client, id } => {
                let mut client = self
                    .client
                    .get_mut(client)
                    .expect("RealCommandExecutor::Client.client has no Client component");
                client.write_packet(&CommandSuggestionsS2c {
                    id: VarInt(id),
                    length: VarInt(
                        (suggestions.span.end().chars() - suggestions.span.begin().chars()) as i32,
                    ),
                    start: VarInt(suggestions.span.begin().chars() as i32),
                    matches: Cow::Owned(
                        suggestions
                            .object
                            .iter()
                            .map(|v| CommandSuggestionsMatch {
                                suggested_match: &v.message,
                                tooltip: v.tooltip.as_ref().map(Cow::Borrowed),
                            })
                            .collect::<Vec<_>>(),
                    ),
                });
            }
        };
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SuggestionsTransaction {
    Client { client: Entity, id: i32 },
    Event { id: u32 },
}

pub trait RawParseSuggestions<'a>: Parse<'a> {
    fn call_suggestions(
        data: &Self::Data,
        real: RealCommandExecutor,
        transaction: SuggestionsTransaction,
        executor: CommandExecutor,
        answer: &mut SuggestionAnswerer,
        suggestions: Self::Suggestions,
        command: String,
        world: &World,
    );
}

pub trait ParseSuggestions<'a>: Parse<'a> {
    fn suggestions(
        data: &Self::Data,
        real: &RealCommandExecutor,
        executor: &CommandExecutor,
        answer: &mut SuggestionAnswerer,
        suggestion: &Self::Suggestions,
        command: &'a str,
        world: &World,
    ) -> Cow<'a, [Suggestion<'a>]>;
}

#[async_trait::async_trait]
pub trait AsyncParseSuggestions<'a>: Parse<'a> {
    type AsyncData: Send;

    /// Creates a data which will be passed then to
    /// [`AsyncParseSuggestions::async_suggestions`] method
    fn create_data(
        data: &Self::Data,
        real: &RealCommandExecutor,
        executor: &CommandExecutor,
        suggestion: &Self::Suggestions,
        command: &'a str,
        world: &World,
    ) -> Self::AsyncData;

    async fn async_suggestions(
        data: &Self::Data,
        async_data: Self::AsyncData,
        real: &RealCommandExecutor,
        executor: &CommandExecutor,
        suggestion: &Self::Suggestions,
        command: &'a str,
    ) -> Cow<'a, [Suggestion<'a>]>;
}
