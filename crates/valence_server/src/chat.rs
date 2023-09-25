use bevy_ecs::prelude::*;
#[cfg(feature = "secure_chat")]
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct CommandExecution {
    pub client: Entity,
    pub command: Box<str>,
    pub timestamp: u64,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub client: Entity,
    pub message: Box<str>,
    pub timestamp: u64,
    #[cfg(feature = "secure_chat")]
    pub message_type: ChatMessageType,
}

#[cfg(feature = "secure_chat")]
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
