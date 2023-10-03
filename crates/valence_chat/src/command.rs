// TODO: Eventually this should be moved to a `valence_commands` crate

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

pub(super) fn build(app: &mut App) {
    app.add_event::<CommandExecutionEvent>();
}

#[derive(Event, Clone, Debug)]
pub struct CommandExecutionEvent {
    pub client: Entity,
    pub command: Box<str>,
    pub timestamp: u64,
    #[cfg(feature = "secure")]
    pub salt: u64,
    #[cfg(feature = "secure")]
    pub argument_signatures: Vec<ArgumentSignature>,
}

#[cfg(feature = "secure")]
#[derive(Clone, Debug)]
pub struct ArgumentSignature {
    pub name: String,
    pub signature: Box<[u8; 256]>,
}
