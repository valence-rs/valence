use std::borrow::Cow;

use bevy_ecs::prelude::{Event, EventReader, EventWriter};
use bevy_ecs::system::Res;
use valence_client::event_loop::PacketEvent;

use crate::command::CommandStorage;
use crate::packet::RequestCommandCompletionsC2s;
use crate::parse::{Suggestion, SuggestionsTransaction, SuggestionsTransactionAnswer};
use crate::reader::{StrLocated, StrReader, StrSpan};

#[derive(Event)]
pub struct CompletionRequestEvent {
    pub transaction: SuggestionsTransaction,
    pub command: String,
}

impl CompletionRequestEvent {
    pub fn reader(&self) -> StrReader {
        StrReader::from_command(&self.command)
    }
}

pub fn completion_request_packet_listener(
    mut event: EventReader<PacketEvent>,
    mut request_event: EventWriter<CompletionRequestEvent>,
) {
    for event in event.iter() {
        if let Some(request) = event.decode::<RequestCommandCompletionsC2s>() {
            request_event.send(CompletionRequestEvent {
                transaction: SuggestionsTransaction::Player {
                    ent: event.client,
                    id: request.id.0,
                },
                command: request.text.to_string(),
            });
        }
    }
}