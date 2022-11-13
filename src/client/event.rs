use std::time::Duration;

use valence_protocol::block::BlockFace;
use valence_protocol::block_pos::BlockPos;
use valence_protocol::entity_meta::Pose;
use valence_protocol::ident::Ident;
use valence_protocol::item::ItemStack;
use valence_protocol::packets::c2s::play::ResourcePackC2s;
use valence_protocol::types::{
    ChatMode, ClickContainerMode, DisplayedSkinParts, EntityInteraction, Hand, MainHand,
};
use valence_protocol::var_int::VarInt;
use vek::Vec3;

use super::Client;
use crate::config::Config;
use crate::entity::{Entity, EntityEvent, EntityId, TrackedData};
use crate::inventory::{Inventory, InventoryDirtyable, SlotId};

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
        interact: EntityInteraction,
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
    PluginMessageReceived {
        channel: Ident<String>,
        data: Vec<u8>,
    },
    ResourcePackStatusChanged(ResourcePackC2s),
    /// The client closed a screen. This occurs when the client closes their
    /// inventory, closes a chest inventory, etc.
    CloseScreen {
        window_id: u8,
    },
    /// The client is attempting to drop 1 of the currently held item.
    DropItem,
    /// The client is attempting to drop a stack of items.
    ///
    /// If the client is in creative mode, the items come from the void, so it
    /// is safe to trust the contents of this event. Otherwise, you may need to
    /// do some validation to make sure items are actually coming from the
    /// user's inventory.
    DropItemStack {
        // TODO: maybe we could add `from_slot_id` to make validation easier
        stack: ItemStack,
    },
    /// The client is in creative mode, and is trying to set it's inventory slot
    /// to a value.
    SetSlotCreative {
        /// The slot number that the client is trying to set.
        slot_id: SlotId,
        /// The contents of the slot.
        slot: Option<ItemStack>,
    },
    /// The client is in survival mode, and is trying to modify an inventory.
    ClickContainer {
        window_id: u8,
        state_id: VarInt,
        /// The slot that was clicked
        slot_id: SlotId,
        /// The type of click that the user performed
        mode: ClickContainerMode,
        /// A list of slot ids and what their contents should be set to.
        ///
        /// It's not safe to blindly trust the contents of this. Servers need to
        /// validate it if they want to prevent item duping.
        slot_changes: Vec<(SlotId, Option<ItemStack>)>,
        /// The item that is now being carried by the user's cursor
        carried_item: Option<ItemStack>,
    },
    RespawnRequest,
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
        ClientEvent::PluginMessageReceived { .. } => {}
        ClientEvent::ResourcePackStatusChanged(_) => {}
        ClientEvent::CloseScreen { window_id } => {
            if let Some(window) = &client.open_inventory {
                if window.window_id == *window_id {
                    client.open_inventory = None;
                }
            }
        }
        ClientEvent::DropItem => {}
        ClientEvent::DropItemStack { .. } => {}
        ClientEvent::SetSlotCreative { slot_id, slot } => {
            let previous_dirty = client.inventory.is_dirty();
            client.inventory.set_slot(*slot_id, slot.clone());
            // HACK: we don't need to mark the inventory as dirty because the
            // client already knows what the updated state of the inventory is.
            client.inventory.mark_dirty(previous_dirty);
        }
        ClientEvent::ClickContainer { .. } => {}
        ClientEvent::RespawnRequest => {}
    }

    entity.set_world(client.world());

    Some(event)
}
