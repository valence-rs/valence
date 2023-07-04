use std::borrow::Cow;
use std::collections::VecDeque;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::Mutex;

use bevy_ecs::prelude::{Entity, Event, EventWriter};
use bevy_ecs::query::WorldQuery;
use bevy_ecs::system::{Query, ReadOnlySystemParam, SystemParam};
use valence_client::Client;
use valence_core::__private::VarInt;
use valence_core::block_pos::BlockPos;
use valence_core::protocol::encode::WritePacket;
use valence_core::protocol::{Decode, Encode};
use valence_core::text::{Color, Text, TextFormat};
use valence_core::translation_key::COMMAND_CONTEXT_HERE;

use crate::packet::CommandSuggestionsS2c;
use crate::reader::{StrLocated, StrReader};

pub type ParseError = StrLocated<Text>;

pub type ParseResult<T> = Result<T, ParseError>;

pub trait Parse<'a>: Sync + Sized + 'a {
    type Data;

    type Suggestions: Default + 'a;

    type Query: ReadOnlySystemParam + 'a;

    fn parse(
        data: &Self::Data,
        suggestions: &mut Self::Suggestions,
        query: &Self::Query,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<Self>;
}

#[derive(Encode, Decode, Clone, Debug, PartialEq)]
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

    pub fn to_static(&self) -> Suggestion<'static> {
        Suggestion {
            message: match self.message {
                Cow::Owned(ref owned) => Cow::Owned(owned.clone()),
                Cow::Borrowed(borrowed) => Cow::Owned(borrowed.to_owned()),
            },
            tooltip: self.tooltip.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SuggestionsTransaction {
    Player { ent: Entity, id: i32 },
    Event { id: u32 },
}

#[derive(SystemParam)]
pub struct SuggestionsTransactionAnswer<'w, 's> {
    client: Query<'w, 's, &'static mut Client>,
    event: EventWriter<'w, CompletedSuggestionEvent>,
}

impl<'w, 's> SuggestionsTransactionAnswer<'w, 's> {
    pub fn answer<'b>(
        &mut self,
        transaction: SuggestionsTransaction,
        suggestions: StrLocated<Cow<'b, [Suggestion<'b>]>>,
    ) {
        match transaction {
            SuggestionsTransaction::Player { ent, id } => {
                self.client
                    .get_mut(ent)
                    .expect("Entity is not a client")
                    .write_packet(&CommandSuggestionsS2c {
                        id: VarInt(id),
                        start: VarInt(suggestions.span.begin().chars() as i32),
                        length: VarInt(
                            (suggestions.span.end().chars() - suggestions.span.begin().chars())
                                as i32,
                        ),
                        matches: suggestions.object,
                    });
            }
            SuggestionsTransaction::Event { id } => {
                self.event.send(CompletedSuggestionEvent {
                    id,
                    suggestions: suggestions
                        .map(|v| v.into_iter().map(|v| v.to_static()).collect()),
                })
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CommandExecutor {
    Entity(Entity),
    Block(Entity, BlockPos),
    Console,
}

pub trait RawParseSuggestions<'a>: Parse<'a> {
    type RawSuggestionsQuery: SystemParam + 'a;

    fn send_suggestions(
        transaction: SuggestionsTransaction,
        answer: &mut SuggestionsTransactionAnswer,
        executor: CommandExecutor,
        query: &mut Self::RawSuggestionsQuery,
        str: String,
        suggestions: Self::Suggestions,
    );
}

pub trait NoSuggestions<'a>: Parse<'a> {}

pub trait ParseSuggestions<'a>: Parse<'a> {
    type SuggestionsQuery: ReadOnlySystemParam;

    fn suggestions(
        executor: CommandExecutor,
        query: &Self::SuggestionsQuery,
        str: String,
        suggestions: Self::Suggestions,
    ) -> StrLocated<Cow<'a, [Suggestion<'a>]>>;
}

#[async_trait::async_trait]
pub trait AsyncParseSuggestions<'a>: Parse<'a> {
    type SuggestionsQuery: ReadOnlySystemParam;

    type AsyncSuggestionsQuery;

    fn async_query(
        executor: CommandExecutor,
        query: &Self::SuggestionsQuery,
    ) -> Self::AsyncSuggestionsQuery;

    async fn suggestions(
        executor: CommandExecutor,
        query: Self::AsyncSuggestionsQuery,
        str: String,
        suggestions: Self::Suggestions,
    ) -> StrLocated<Cow<'a, [Suggestion<'a>]>>;
}

#[derive(Event, Clone, Debug, PartialEq)]
pub struct CompletedSuggestionEvent {
    pub id: u32,
    pub suggestions: StrLocated<Vec<Suggestion<'static>>>,
}

#[macro_export]
macro_rules! suggestions_impl {
    (!$ty: ty) => {
        impl<'a> $crate::parse::RawParseSuggestions<'a> for $ty {
            type RawSuggestionsQuery = ();

            fn send_suggestions(
                _transaction: $crate::parse::SuggestionsTransaction,
                _answer: &mut $crate::parse::SuggestionsTransactionAnswer,
                _executor: $crate::parse::CommandExecutor,
                _query: &mut Self::RawSuggestionsQuery,
                _str: String,
                _suggestions: Self::Suggestions,
            ) {};
        }

    };
    (async $ty:ty) => {
        compile_error!("async is not implemented, yet");
    };
    ($ty:ty) => {
        impl<'a> $crate::parse::RawParseSuggestions<'a> for $ty
        where
            $ty: $crate::parse::ParseSuggestions<'a>,
        {
            type RawSuggestionsQuery =
                <$ty as $crate::parse::ParseSuggestions<'a>>::SuggestionsQuery;

            fn send_suggestions(
                transaction: $crate::parse::SuggestionsTransaction,
                answer: &mut $crate::parse::SuggestionsTransactionAnswer,
                executor: $crate::parse::CommandExecutor,
                query: &mut Self::RawSuggestionsQuery,
                str: String,
                suggestions: Self::Suggestions,
            ) {
                answer.answer(transaction, <$ty>::suggestions(executor, &query, str, suggestions));
            }
        }
    };
}

pub const BRIGADIER_LIKE_ERROR_MESSAGE: bool = true;

pub fn parse_error_message(reader: &StrReader, error: ParseError) -> Text {
    let ParseError {
        span,
        object: error,
    } = error;

    if BRIGADIER_LIKE_ERROR_MESSAGE {
        let mut reader = reader.clone();

        // SAFETY: span is valid
        unsafe { reader.set_cursor(span.end()) };

        Text::text("")
            .color(Color::RED)
            .add_child(error)
            .add_child(Text::text("\n"))
            .add_child(Text::text(reader.used_str().to_string()))
            .add_child(Text::translate(COMMAND_CONTEXT_HERE, vec![]))
            .add_child(Text::text(reader.remaining_str().to_string()))
    } else {
        // ParseError contains more informative span than brigadier does, so we can
        // actually give error's place more accurate

        let (left, right) = reader.str().split_at(span.begin().bytes());
        let (middle, right) = right.split_at(span.end().bytes());

        Text::text("")
            .add_child(error)
            .add_child(Text::text("\n"))
            .add_child(Text::text(left.to_string()))
            .add_child(Text::text(middle.to_string()).color(Color::RED))
            .add_child(Text::text(right.to_string()))
    }
}
