use std::borrow::Cow;
use std::collections::HashSet;

use bevy_app::App;
use bevy_ecs::prelude::{Component, Entity, Event, EventReader, EventWriter};
use bevy_ecs::schedule::SystemConfigs;
use bevy_ecs::system::{Query, Res, Resource};
use valence_client::event_loop::PacketEvent;
use valence_client::message::SendMessage;
use valence_client::Client;
use valence_core::protocol::packet::chat::CommandExecutionC2s;
use valence_core::protocol::packet::command::{Node, Parser};
use valence_core::text::Text;
use valence_core::translation_key::COMMAND_UNKNOWN_COMMAND;

use crate::parse::parse_error_message;
use crate::reader::{StrCursor, StrLocated, StrReader, StrSpan};

pub trait Command {
    fn name() -> String;

    fn build(app: &mut App);
}

#[derive(Resource, Default, Clone, Debug)]
pub struct CommandStorage {
    commands: HashSet<String>,
}

impl CommandStorage {
    /// Won't send messages that this command doesn't exist. It doesn't register
    /// any systems
    pub fn register_command(&mut self, name: String) {
        self.commands.insert(name);
    }

    pub fn is_registered(&self, name: &str) -> bool {
        self.commands.contains(name)
    }
}

pub trait RegisterCommand {
    fn register_command<C: Command>(&mut self) -> &mut Self;
}

impl RegisterCommand for App {
    fn register_command<C: Command>(&mut self) -> &mut Self {
        self.world
            .resource_mut::<CommandStorage>()
            .register_command(C::name());
        C::build(self);
        self
    }
}

#[derive(Event)]
pub struct CommandExecutionEvent {
    pub client: Entity,
    whole: String,
    backslash: usize,
    name: usize,
}

impl CommandExecutionEvent {
    pub fn new(client: Entity, command: String) -> Self {
        let name = command.find(' ').unwrap_or(command.len());

        Self {
            client,
            backslash: if command.starts_with('/') {
                '/'.len_utf8()
            } else {
                0
            },
            whole: command,
            name,
        }
    }

    pub fn whole(&self) -> &str {
        // SAFETY: backslash is always a valid byte index
        unsafe { self.whole.get_unchecked(self.backslash..) }
    }

    pub fn name(&self) -> &str {
        // SAFETY: name is always a valid byte index
        unsafe { self.whole.get_unchecked(self.backslash..self.name) }
    }

    pub fn reader(&self) -> StrReader {
        let mut reader = StrReader::new(self.whole());

        let cursor = self.name;

        // SAFETY: cursor is valid
        let chars = unsafe { self.whole.get_unchecked(0..cursor) }
            .chars()
            .count();

        // SAFETY: cursor is valid
        unsafe { reader.set_cursor(StrCursor::new(chars, cursor)) }

        reader
    }

    fn name_span(&self) -> StrSpan {
        StrSpan::new(
            StrCursor::new(self.backslash, self.backslash),
            StrCursor::new(self.name, self.name().chars().count() + self.backslash),
        )
    }
}

pub fn command_event(
    mut packets: EventReader<PacketEvent>,
    mut execution: EventWriter<CommandExecutionEvent>,
) {
    for packet_event in packets.iter() {
        if let Some(packet) = packet_event.decode::<CommandExecutionC2s>() {
            execution.send(CommandExecutionEvent::new(
                packet_event.client,
                packet.command.to_string(),
            ));
        }
    }
}

pub fn check_command_not_found(
    mut execution: EventReader<CommandExecutionEvent>,
    storage: Res<CommandStorage>,
    mut client: Query<&mut Client>,
) {
    for execution in execution.iter() {
        if !storage.is_registered(execution.name()) {
            let Ok(mut client) = client.get_mut(execution.client) else {
                continue;
            };

            client.send_chat_message(parse_error_message(
                &StrReader::new(execution.whole()),
                StrLocated::new(
                    execution.name_span(),
                    Text::translate(COMMAND_UNKNOWN_COMMAND, vec![]),
                ),
            ));
        }
    }
}
