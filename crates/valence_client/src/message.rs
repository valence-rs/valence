// TODO: delete this module in favor of valence_chat.

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_packet::protocol::encode::WritePacket;
use valence_core::text::IntoText;
use valence_packet::packets::play::{ChatMessageC2s, GameMessageS2c};

use crate::event_loop::{EventLoopPreUpdate, PacketEvent};

pub(super) fn build(app: &mut App) {
    app.add_event::<ChatMessageEvent>()
        .add_systems(EventLoopPreUpdate, handle_chat_message);
}

pub trait SendMessage {
    /// Sends a system message visible in the chat.
    fn send_chat_message<'a>(&mut self, msg: impl IntoText<'a>);
    /// Displays a message in the player's action bar (text above the hotbar).
    fn send_action_bar_message<'a>(&mut self, msg: impl IntoText<'a>);
}

impl<T: WritePacket> SendMessage for T {
    fn send_chat_message<'a>(&mut self, msg: impl IntoText<'a>) {
        self.write_packet(&GameMessageS2c {
            chat: msg.into_cow_text(),
            overlay: false,
        });
    }

    fn send_action_bar_message<'a>(&mut self, msg: impl IntoText<'a>) {
        self.write_packet(&GameMessageS2c {
            chat: msg.into_cow_text(),
            overlay: true,
        });
    }
}

#[derive(Event, Clone, Debug)]
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
