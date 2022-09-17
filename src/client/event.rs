use std::time::Duration;

use vek::Vec3;

use super::Client;
use crate::block_pos::BlockPos;
use crate::config::Config;
use crate::entity::types::Pose;
use crate::entity::{Entity, EntityEvent, EntityId, TrackedData};
pub use crate::protocol::packets::c2s::play::{
    BlockFace, ChatMode, DisplayedSkinParts, Hand, MainHand, ResourcePackC2s as ResourcePackStatus,
};
pub use crate::protocol::packets::s2c::play::GameMode;
use crate::protocol::VarInt;

/// Represents an action performed by a client.
///
/// Client events can be obtained from
/// [`pop_event`](super::Client::pop_event).
///
/// # Event Validation
///
/// [`Client`](super::Client) makes no attempt to validate events against the
/// expected rules for players. Malicious clients can teleport through walls,
/// interact with distant entities, sneak and sprint backwards, break
/// bedrock in survival mode, etc.
///
/// It is best to think of events from clients as _requests_ to interact with
/// the server. It is then your responsibility to decide if the request should
/// be honored.
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
    InteractWithBlock {
        /// The hand that was used
        hand: Hand,
        /// The location of the block that was interacted with
        location: BlockPos,
        /// The face of the block that was clicked
        face: BlockFace,
        /// The pos inside of the block that was clicked on
        cursor_pos: Vec3<f32>,
        /// Whether or not the player's head is inside a block
        head_inside_block: bool,
        /// Sequence number
        sequence: VarInt,
    },
    ResourcePackStatusChanged(ResourcePackStatus),
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

/// Pops one event from the event queue of `client` and expresses the event in a
/// reasonable way using `entity`. For instance, movement events are expressed
/// by changing the entity's position to match the received position. Rotation
/// events rotate the entity. etc.
///
/// This function's primary purpose is to reduce boilerplate code in the
/// examples, but it can be used as a quick way to get started in your own code.
/// The precise behavior of this function is left unspecified and is subject to
/// change.
///
/// The popped event is returned unmodified. `None` is returned if there are no
/// more events in `client`.
pub fn handle_event_default<C: Config>(
    client: &mut Client<C>,
    entity: &mut Entity<C>,
) -> Option<ClientEvent> {
    let event = client.pop_event()?;

    match &event {
        ClientEvent::ChatMessage { .. } => {}
        ClientEvent::SettingsChanged {
            view_distance,
            main_hand,
            displayed_skin_parts,
            ..
        } => {
            client.set_view_distance(*view_distance);

            let player = client.player_mut();

            player.set_cape(displayed_skin_parts.cape());
            player.set_jacket(displayed_skin_parts.jacket());
            player.set_left_sleeve(displayed_skin_parts.left_sleeve());
            player.set_right_sleeve(displayed_skin_parts.right_sleeve());
            player.set_left_pants_leg(displayed_skin_parts.left_pants_leg());
            player.set_right_pants_leg(displayed_skin_parts.right_pants_leg());
            player.set_hat(displayed_skin_parts.hat());
            player.set_main_arm(*main_hand as u8);

            if let TrackedData::Player(player) = entity.data_mut() {
                player.set_cape(displayed_skin_parts.cape());
                player.set_jacket(displayed_skin_parts.jacket());
                player.set_left_sleeve(displayed_skin_parts.left_sleeve());
                player.set_right_sleeve(displayed_skin_parts.right_sleeve());
                player.set_left_pants_leg(displayed_skin_parts.left_pants_leg());
                player.set_right_pants_leg(displayed_skin_parts.right_pants_leg());
                player.set_hat(displayed_skin_parts.hat());
                player.set_main_arm(*main_hand as u8);
            }
        }
        ClientEvent::MovePosition {
            position,
            on_ground,
        } => {
            entity.set_position(*position);
            entity.set_on_ground(*on_ground);
        }
        ClientEvent::MovePositionAndRotation {
            position,
            yaw,
            pitch,
            on_ground,
        } => {
            entity.set_position(*position);
            entity.set_yaw(*yaw);
            entity.set_head_yaw(*yaw);
            entity.set_pitch(*pitch);
            entity.set_on_ground(*on_ground);
        }
        ClientEvent::MoveRotation {
            yaw,
            pitch,
            on_ground,
        } => {
            entity.set_yaw(*yaw);
            entity.set_head_yaw(*yaw);
            entity.set_pitch(*pitch);
            entity.set_on_ground(*on_ground);
        }
        ClientEvent::MoveOnGround { on_ground } => {
            entity.set_on_ground(*on_ground);
        }
        ClientEvent::MoveVehicle { .. } => {}
        ClientEvent::StartSneaking => {
            if let TrackedData::Player(player) = entity.data_mut() {
                if player.get_pose() == Pose::Standing {
                    player.set_pose(Pose::Sneaking);
                }
            }
        }
        ClientEvent::StopSneaking => {
            if let TrackedData::Player(player) = entity.data_mut() {
                if player.get_pose() == Pose::Sneaking {
                    player.set_pose(Pose::Standing);
                }
            }
        }
        ClientEvent::StartSprinting => {
            if let TrackedData::Player(player) = entity.data_mut() {
                player.set_sprinting(true);
            }
        }
        ClientEvent::StopSprinting => {
            if let TrackedData::Player(player) = entity.data_mut() {
                player.set_sprinting(false);
            }
        }
        ClientEvent::StartJumpWithHorse { .. } => {}
        ClientEvent::StopJumpWithHorse => {}
        ClientEvent::LeaveBed => {}
        ClientEvent::OpenHorseInventory => {}
        ClientEvent::StartFlyingWithElytra => {}
        ClientEvent::ArmSwing(hand) => {
            entity.push_event(match hand {
                Hand::Main => EntityEvent::SwingMainHand,
                Hand::Off => EntityEvent::SwingOffHand,
            });
        }
        ClientEvent::InteractWithEntity { .. } => {}
        ClientEvent::SteerBoat { .. } => {}
        ClientEvent::Digging { .. } => {}
        ClientEvent::InteractWithBlock { .. } => {}
        ClientEvent::ResourcePackStatusChanged(_) => {}
    }

    entity.set_world(client.world());

    Some(event)
}
