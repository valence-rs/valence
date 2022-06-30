use std::time::Duration;

use vek::Vec3;

use crate::protocol::packets::play::c2s::BlockFace;
pub use crate::protocol::packets::play::c2s::{ChatMode, DisplayedSkinParts, Hand, MainHand};
pub use crate::protocol::packets::play::s2c::GameMode;
use crate::{BlockPos, EntityId};

#[derive(Debug)]
pub enum Event {
    ChatMessage {
        message: String,
        timestamp: Duration,
    },
    /// Settings were changed. The value in this variant is the previous client
    /// settings.
    SettingsChanged(Option<Settings>),
    /// The client has moved. The values in this
    /// variant are the _previous_ position and look.
    Movement {
        position: Vec3<f64>,
        yaw: f32,
        pitch: f32,
        on_ground: bool,
    },
    InteractWithEntity {
        /// The ID of the entity being interacted with.
        id: EntityId,
        /// If the client was sneaking during the interaction.
        sneaking: bool,
        /// The type of interaction that occurred.
        typ: InteractWithEntity,
    },
    SteerBoat {
        left_paddle_turning: bool,
        right_paddle_turning: bool,
    },
    Digging(Digging),
}

#[derive(Clone, PartialEq, Debug)]
pub struct Settings {
    /// e.g. en_US
    pub locale: String,
    /// The client side render distance, in chunks.
    ///
    /// The value is always in `2..=32`.
    pub view_distance: u8,
    pub chat_mode: ChatMode,
    /// `true` if the client has chat colors enabled, `false` otherwise.
    pub chat_colors: bool,
    pub main_hand: MainHand,
    pub displayed_skin_parts: DisplayedSkinParts,
    pub allow_server_listings: bool,
}

#[derive(Clone, PartialEq, Debug)]
pub enum InteractWithEntity {
    Interact(Hand),
    InteractAt { target: Vec3<f32>, hand: Hand },
    Attack,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Digging {
    pub status: DiggingStatus,
    pub position: BlockPos,
    pub face: BlockFace,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DiggingStatus {
    Start,
    Cancel,
    Finish,
}
