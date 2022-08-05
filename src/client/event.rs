use std::time::Duration;

use vek::Vec3;

use crate::block_pos::BlockPos;
use crate::entity::EntityId;
use crate::protocol_inner::packets::c2s::play::BlockFace;
pub use crate::protocol_inner::packets::c2s::play::{ChatMode, DisplayedSkinParts, Hand, MainHand};
pub use crate::protocol_inner::packets::s2c::play::GameMode;

/// Represents an action performed by a client.
///
/// Client events can be obtained from
/// [`pop_event`](crate::client::Client::pop_event).
#[derive(Debug)]
pub enum ClientEvent {
    /// A regular message was sent to the chat.
    ChatMessage {
        /// The content of the message
        message: String,
        /// The time the message was sent.
        timestamp: Duration,
    },
    /// Settings were changed. This is always sent once after joining by the
    /// vanilla client.
    SettingsChanged {
        /// e.g. en_US
        locale: String,
        /// The client side render distance, in chunks.
        ///
        /// The value is always in `2..=32`.
        view_distance: u8,
        chat_mode: ChatMode,
        /// `true` if the client has chat colors enabled, `false` otherwise.
        chat_colors: bool,
        main_hand: MainHand,
        displayed_skin_parts: DisplayedSkinParts,
        allow_server_listings: bool,
    },
    MovePosition {
        position: Vec3<f64>,
        on_ground: bool,
    },
    MovePositionAndRotation {
        position: Vec3<f64>,
        yaw: f32,
        pitch: f32,
        on_ground: bool,
    },
    MoveRotation {
        yaw: f32,
        pitch: f32,
        on_ground: bool,
    },
    MoveOnGround {
        on_ground: bool,
    },
    MoveVehicle {
        position: Vec3<f64>,
        yaw: f32,
        pitch: f32,
    },
    StartSneaking,
    StopSneaking,
    StartSprinting,
    StopSprinting,
    /// A jump while on a horse started.
    StartJumpWithHorse {
        /// The power of the horse jump.
        jump_boost: u8,
    },
    /// A jump while on a horse stopped.
    StopJumpWithHorse,
    /// The client left a bed.
    LeaveBed,
    /// The inventory was opened while on a horse.
    OpenHorseInventory,
    StartFlyingWithElytra,
    ArmSwing(Hand),
    /// Left or right click interaction with an entity's hitbox.
    InteractWithEntity {
        /// The ID of the entity being interacted with.
        id: EntityId,
        /// If the client was sneaking during the interaction.
        sneaking: bool,
        /// The kind of interaction that occurred.
        kind: InteractWithEntityKind,
    },
    SteerBoat {
        left_paddle_turning: bool,
        right_paddle_turning: bool,
    },
    Digging {
        /// The kind of digging event this is.
        status: DiggingStatus,
        /// The position of the block being broken.
        position: BlockPos,
        /// The face of the block being broken.
        face: BlockFace,
    },
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
pub enum InteractWithEntityKind {
    Interact(Hand),
    InteractAt { target: Vec3<f32>, hand: Hand },
    Attack,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DiggingStatus {
    /// The client started digging a block.
    Start,
    /// The client stopped digging a block before it was fully broken.
    Cancel,
    /// The client finished digging a block successfully.
    Finish,
}
