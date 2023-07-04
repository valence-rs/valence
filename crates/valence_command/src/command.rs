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

use crate::parse::{parse_error_message, CommandExecutor};
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

    pub fn commands(&self) -> impl Iterator<Item = &String> {
        self.commands.iter()
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
    pub executor: CommandExecutor,
    pub command: String,
}

impl CommandExecutionEvent {
    pub fn new(executor: CommandExecutor, command: String) -> Self {
        Self { executor, command }
    }

    pub fn reader(&self) -> StrReader {
        StrReader::from_command(&self.command)
    }
}

pub fn command_event(
    mut packets: EventReader<PacketEvent>,
    mut execution: EventWriter<CommandExecutionEvent>,
) {
    for packet_event in packets.iter() {
        if let Some(packet) = packet_event.decode::<CommandExecutionC2s>() {
            execution.send(CommandExecutionEvent::new(
                CommandExecutor::Entity(packet_event.client),
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
        let mut reader = execution.reader();
        let begin = reader.cursor();
        if !storage.is_registered(reader.read_unquoted_str()) {
            let error_message = parse_error_message(
                &reader,
                StrLocated::new(
                    StrSpan::new(begin, reader.cursor()),
                    Text::translate(COMMAND_UNKNOWN_COMMAND, vec![]),
                ),
            );

            match execution.executor {
                CommandExecutor::Entity(entity) => {
                    let Ok(mut client) = client.get_mut(entity) else {
                        continue;
                    };

                    client.send_chat_message(error_message);
                }
                _ => todo!(),
            }
        }
    }
}
