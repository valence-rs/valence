// TODO: delete this module in favor of valence_chat.

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_core::protocol::encode::WritePacket;
use valence_core::protocol::packet::chat::{ChatMessageC2s, GameMessageS2c};
use valence_core::text::Text;

use crate::event_loop::{EventLoopSchedule, EventLoopSet, PacketEvent};

pub(super) fn build(app: &mut App) {
    app.add_event::<ChatMessageEvent>().add_system(
        handle_chat_message
            .in_schedule(EventLoopSchedule)
            .in_base_set(EventLoopSet::PreUpdate),
    );
}

pub trait SendMessage {
    /// Sends a system message visible in the chat.
    fn send_chat_message(&mut self, msg: impl Into<Text>);
    /// Displays a message in the player's action bar (text above the hotbar).
    fn send_action_bar_message(&mut self, msg: impl Into<Text>);
}

impl<T: WritePacket> SendMessage for T {
    fn send_chat_message(&mut self, msg: impl Into<Text>) {
        self.write_packet(&GameMessageS2c {
            chat: msg.into().into(),
            overlay: false,
        });
    }

    fn send_action_bar_message(&mut self, msg: impl Into<Text>) {
        self.write_packet(&GameMessageS2c {
            chat: msg.into().into(),
            overlay: true,
        });
    }
}

#[derive(Clone, Debug)]
pub struct ChatMessageEvent {
    pub client: Entity,
    pub message: Box<str>,
    pub timestamp: u64,
}

pub fn handle_chat_message(
    mut packets: EventReader<PacketEvent>,
    mut events: EventWriter<ChatMessageEvent>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<ChatMessageC2s>() {
            events.send(ChatMessageEvent {
                client: packet.client,
                message: pkt.message.into(),
                timestamp: pkt.timestamp,
            });
        }
    }
}
