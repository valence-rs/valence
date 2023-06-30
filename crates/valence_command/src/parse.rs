use std::borrow::Cow;
use std::collections::VecDeque;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::Mutex;

use bevy_ecs::prelude::{Entity, EventWriter, Event};
use bevy_ecs::query::WorldQuery;
use bevy_ecs::system::{Query, ReadOnlySystemParam, SystemParam};
use valence_client::Client;
use valence_core::block_pos::BlockPos;
use valence_core::protocol::{Decode, Encode};
use valence_core::text::{Color, Text, TextFormat};
use valence_core::translation_key::COMMAND_CONTEXT_HERE;

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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SuggestionsTransaction {
    Player { ent: Entity, id: i32 },
    Event { id: u32 },
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
    pub suggestions: Vec<Suggestion<'static>>,
}

#[macro_export]
macro_rules! suggestions_impl {
    (!$ty: ty) => {
        impl<'a> $crate::parse::RawParseSuggestions<'a> for $ty {
            type RawSuggestionsQuery = ();

            fn send_suggestions(
                _transaction: $crate::parse::SuggestionsTransaction,
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
            type RawSuggestionsQuery = (
                bevy_ecs::prelude::Query<'a, 'a, &'static mut valence_client::Client>,
                bevy_ecs::prelude::EventWriter<'a, $crate::parse::CompletedSuggestionEvent>,
                <$ty as $crate::parse::ParseSuggestions<'a>>::SuggestionsQuery,
            );

            fn send_suggestions(
                transaction: $crate::parse::SuggestionsTransaction,
                executor: $crate::parse::CommandExecutor,
                query: &mut Self::RawSuggestionsQuery,
                str: String,
                suggestions: Self::Suggestions,
            ) {
                let result = <$ty>::suggestions(executor, &query.2, str, suggestions);

                match transaction {
                    $crate::parse::SuggestionsTransaction::Player { ent, id } => {
                        if let Ok(mut client) = query.0.get_mut(ent) {
                            let id = valence_core::protocol::var_int::VarInt(id);
                            let start = valence_core::protocol::var_int::VarInt(result.span.begin().chars() as i32);
                            let length = valence_core::protocol::var_int::VarInt((result.span.end().chars() - result.span.begin().chars()) as i32);
                            let matches = result.object;
                            <valence_client::Client as valence_core::protocol::encode::WritePacket>::write_packet(&mut client, &$crate::packet::CommandSuggestionsS2c {
                                id,
                                start,
                                length,
                                matches,
                            });
                        }
                    }
                    $crate::parse::SuggestionsTransaction::Event { id } => {
                        todo!()
                    }
                }
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
            .add_child(error.color(Color::RED))
            .add_child(Text::text("\n"))
            .add_child(Text::text(reader.used_str().to_string()).color(Color::RED))
            .add_child(Text::translate(COMMAND_CONTEXT_HERE, vec![]).color(Color::RED))
            .add_child(Text::text(reader.remaining_str().to_string()).color(Color::RED))
    } else {
        // ParseError contains more informative span than brigadier does, so we can
        // actually give error place more accurate
        todo!()
    }
}
