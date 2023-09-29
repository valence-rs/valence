use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_protocol::encode::WritePacket;
use valence_protocol::packets::play::GameMessageS2c;
use valence_protocol::text::IntoText;

#[cfg(feature = "secure")]
use uuid::Uuid;

pub(super) fn build(app: &mut App) {
    app.add_event::<CommandExecutionEvent>()
        .add_event::<ChatMessageEvent>();
}

pub trait SendMessage {
    /// Sends a system message visible in the chat.
    fn send_game_message<'a>(&mut self, msg: impl IntoText<'a>);
    /// Displays a message in the player's action bar (text above the hotbar).
    fn send_action_bar_message<'a>(&mut self, msg: impl IntoText<'a>);
}

impl<T: WritePacket> SendMessage for T {
    fn send_game_message<'a>(&mut self, msg: impl IntoText<'a>) {
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
pub struct CommandExecutionEvent {
    pub client: Entity,
    pub command: Box<str>,
    pub timestamp: u64,
}

#[derive(Event, Clone, Debug)]
pub struct ChatMessageEvent {
    pub client: Entity,
    pub message: Box<str>,
    pub timestamp: u64,
    #[cfg(feature = "secure")]
    pub message_type: ChatMessageType,
}

#[cfg(feature = "secure")]
#[derive(Clone, Debug)]
pub enum ChatMessageType {
    Signed {
        salt: u64,
        signature: Box<[u8; 256]>,
        message_index: i32,
        sender: Uuid,
        last_seen: Vec<[u8; 256]>,
    },
    Unsigned,
}