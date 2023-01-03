use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ShouldRun;
use uuid::Uuid;
use valence_protocol::types::{
    ChatMode, ClickContainerMode, CommandBlockMode, Difficulty, DisplayedSkinParts,
    EntityInteraction, Hand, MainHand, RecipeBookId, StructureBlockAction, StructureBlockFlags,
    StructureBlockMirror, StructureBlockMode, StructureBlockRotation,
};
use valence_protocol::{BlockFace, BlockPos, Ident, ItemStack};

#[derive(Clone, Debug)]
pub struct QueryBlockEntity {
    pub client: Entity,
    pub position: BlockPos,
    pub transaction_id: i32,
}

#[derive(Clone, Debug)]
pub struct ChangeDifficulty {
    pub client: Entity,
    pub difficulty: Difficulty,
}

#[derive(Clone, Debug)]
pub struct MessageAcknowledgment {
    pub client: Entity,
    pub last_seen: Vec<(Uuid, Box<[u8]>)>,
    pub last_received: Option<(Uuid, Box<[u8]>)>,
}

#[derive(Clone, Debug)]
pub struct ChatCommand {
    pub client: Entity,
    pub command: Box<str>,
    pub timestamp: u64,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub client: Entity,
    pub message: Box<str>,
    pub timestamp: u64,
}

#[derive(Clone, Debug)]
pub struct ChatPreview {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct PerformRespawn {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct RequestStats {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct UpdateSettings {
    pub client: Entity,
    /// e.g. en_US
    pub locale: Box<str>,
    /// The client side render distance, in chunks.
    ///
    /// The value is always in `2..=32`.
    pub view_distance: u8,
    pub chat_mode: ChatMode,
    /// `true` if the client has chat colors enabled, `false` otherwise.
    pub chat_colors: bool,
    pub displayed_skin_parts: DisplayedSkinParts,
    pub main_hand: MainHand,
    pub enable_text_filtering: bool,
    pub allow_server_listings: bool,
}

#[derive(Clone, Debug)]
pub struct CommandSuggestionsRequest {
    pub client: Entity,
    pub transaction_id: i32,
    pub text: Box<str>,
}

#[derive(Clone, Debug)]
pub struct ClickContainerButton {
    pub client: Entity,
    pub window_id: i8,
    pub button_id: i8,
}

#[derive(Clone, Debug)]
pub struct ClickContainer {
    pub client: Entity,
    pub window_id: u8,
    pub state_id: i32,
    pub slot_id: i16,
    pub button: i8,
    pub mode: ClickContainerMode,
    pub slot_changes: Vec<(i16, Option<ItemStack>)>,
    pub carried_item: Option<ItemStack>,
}

#[derive(Clone, Debug)]
pub struct CloseContainer {
    pub client: Entity,
    pub window_id: i8,
}

#[derive(Clone, Debug)]
pub struct PluginMessage {
    pub client: Entity,
    pub channel: Ident<Box<str>>,
    pub data: Box<[u8]>,
}

#[derive(Clone, Debug)]
pub struct EditBook {
    pub slot: i32,
    pub entries: Vec<Box<str>>,
    pub title: Option<Box<str>>,
}

#[derive(Clone, Debug)]
pub struct QueryEntity {
    pub client: Entity,
    pub transaction_id: i32,
    pub entity_id: i32,
}

/// Left or right click interaction with an entity's hitbox.
#[derive(Clone, Debug)]
pub struct InteractWithEntity {
    pub client: Entity,
    /// The raw ID of the entity being interacted with.
    pub entity_id: i32,
    /// If the client was sneaking during the interaction.
    pub sneaking: bool,
    /// The kind of interaction that occurred.
    pub interact: EntityInteraction,
}

#[derive(Clone, Debug)]
pub struct JigsawGenerate {
    pub client: Entity,
    pub position: BlockPos,
    pub levels: i32,
    pub keep_jigsaws: bool,
}

#[derive(Clone, Debug)]
pub struct LockDifficulty {
    pub client: Entity,
    pub locked: bool,
}

#[derive(Clone, Debug)]
pub struct SetPlayerPosition {
    pub client: Entity,
    pub position: [f64; 3],
    pub on_ground: bool,
}

#[derive(Clone, Debug)]
pub struct SetPlayerPositionAndRotation {
    pub client: Entity,
    pub position: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

#[derive(Clone, Debug)]
pub struct SetPlayerRotation {
    pub client: Entity,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

#[derive(Clone, Debug)]
pub struct SetPlayerOnGround {
    pub client: Entity,
    pub on_ground: bool,
}

#[derive(Clone, Debug)]
pub struct MoveVehicle {
    pub client: Entity,
    pub position: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Clone, Debug)]
pub struct StartSneaking {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct StopSneaking {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct LeaveBed {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct StartSprinting {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct StopSprinting {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct StartJumpWithHorse {
    pub client: Entity,
    /// The power of the horse jump in `0..=100`.
    pub jump_boost: u8,
}

#[derive(Clone, Debug)]
pub struct StopJumpWithHorse {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct OpenHorseInventory {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct StartFlyingWithElytra {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct PaddleBoat {
    pub client: Entity,
    pub left_paddle_turning: bool,
    pub right_paddle_turning: bool,
}

#[derive(Clone, Debug)]
pub struct PickItem {
    pub client: Entity,
    pub slot_to_use: i32,
}

#[derive(Clone, Debug)]
pub struct PlaceRecipe {
    pub client: Entity,
    pub window_id: i8,
    pub recipe: Ident<Box<str>>,
    pub make_all: bool,
}

#[derive(Clone, Debug)]
pub struct StopFlying {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct StartFlying {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct StartDigging {
    pub client: Entity,
    pub position: BlockPos,
    pub face: BlockFace,
    pub sequence: i32,
}

#[derive(Clone, Debug)]
pub struct CancelDigging {
    pub client: Entity,
    pub position: BlockPos,
    pub face: BlockFace,
    pub sequence: i32,
}

#[derive(Clone, Debug)]
pub struct FinishDigging {
    pub client: Entity,
    pub position: BlockPos,
    pub face: BlockFace,
    pub sequence: i32,
}

#[derive(Clone, Debug)]
pub struct DropItem {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct DropItemStack {
    pub client: Entity,
}

/// Eating food, pulling back bows, using buckets, etc.
#[derive(Clone, Debug)]
pub struct UpdateHeldItemState {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct SwapItemInHand {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct PlayerInput {
    pub client: Entity,
    pub sideways: f32,
    pub forward: f32,
    pub jump: bool,
    pub unmount: bool,
}

#[derive(Clone, Debug)]
pub struct Pong {
    pub client: Entity,
    pub id: i32,
}

#[derive(Clone, Debug)]
pub struct PlayerSession {
    pub client: Entity,
    pub session_id: Uuid,
    pub expires_at: i64,
    pub public_key_data: Box<[u8]>,
    pub key_signature: Box<[u8]>,
}

#[derive(Clone, Debug)]
pub struct ChangeRecipeBookSettings {
    pub client: Entity,
    pub book_id: RecipeBookId,
    pub book_open: bool,
    pub filter_active: bool,
}

#[derive(Clone, Debug)]
pub struct SetSeenRecipe {
    pub client: Entity,
    pub recipe_id: Ident<Box<str>>,
}

#[derive(Clone, Debug)]
pub struct RenameItem {
    pub client: Entity,
    pub name: Box<str>,
}

#[derive(Clone, Debug)]
pub struct ResourcePackLoaded {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct ResourcePackDeclined {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct ResourcePackFailedDownload {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct ResourcePackAccepted {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct OpenAdvancementTab {
    pub client: Entity,
    pub tab_id: Ident<Box<str>>,
}

#[derive(Clone, Debug)]
pub struct CloseAdvancementScreen {
    pub client: Entity,
}

#[derive(Clone, Debug)]
pub struct SelectTrade {
    pub client: Entity,
    pub slot: i32,
}

#[derive(Clone, Debug)]
pub struct SetBeaconEffect {
    pub client: Entity,
    pub primary_effect: Option<i32>,
    pub secondary_effect: Option<i32>,
}

#[derive(Clone, Debug)]
pub struct SetHeldItem {
    pub client: Entity,
    pub slot: i16,
}

#[derive(Clone, Debug)]
pub struct ProgramCommandBlock {
    pub client: Entity,
    pub position: BlockPos,
    pub command: Box<str>,
    pub mode: CommandBlockMode,
    pub track_output: bool,
    pub conditional: bool,
    pub automatic: bool,
}

#[derive(Clone, Debug)]
pub struct ProgramCommandBlockMinecart {
    pub client: Entity,
    pub entity_id: i32,
    pub command: Box<str>,
    pub track_output: bool,
}

#[derive(Clone, Debug)]
pub struct SetCreativeModeSlot {
    pub client: Entity,
    pub slot: i16,
    pub clicked_item: Option<ItemStack>,
}

#[derive(Clone, Debug)]
pub struct ProgramJigsawBlock {
    pub client: Entity,
    pub position: BlockPos,
    pub name: Ident<Box<str>>,
    pub target: Ident<Box<str>>,
    pub pool: Ident<Box<str>>,
    pub final_state: Box<str>,
    pub joint_type: Box<str>,
}

#[derive(Clone, Debug)]
pub struct ProgramStructureBlock {
    pub client: Entity,
    pub position: BlockPos,
    pub action: StructureBlockAction,
    pub mode: StructureBlockMode,
    pub name: Box<str>,
    pub offset_xyz: [i8; 3],
    pub size_xyz: [i8; 3],
    pub mirror: StructureBlockMirror,
    pub rotation: StructureBlockRotation,
    pub metadata: Box<str>,
    pub integrity: f32,
    pub seed: i64,
    pub flags: StructureBlockFlags,
}

#[derive(Clone, Debug)]
pub struct UpdateSign {
    pub client: Entity,
    pub position: BlockPos,
    pub lines: [Box<str>; 4],
}

#[derive(Clone, Debug)]
pub struct SwingArm {
    pub client: Entity,
    pub hand: Hand,
}

#[derive(Clone, Debug)]
pub struct TeleportToEntity {
    pub client: Entity,
    pub target: Uuid,
}

#[derive(Clone, Debug)]
pub struct UseItemOnBlock {
    pub client: Entity,
    /// The hand that was used
    pub hand: Hand,
    /// The location of the block that was interacted with
    pub position: BlockPos,
    /// The face of the block that was clicked
    pub face: BlockFace,
    /// The position inside of the block that was clicked on
    pub cursor_pos: [f32; 3],
    /// Whether or not the player's head is inside a block
    pub head_inside_block: bool,
    /// Sequence number for synchronization
    pub sequence: i32,
}

#[derive(Clone, Debug)]
pub struct UseItem {
    pub client: Entity,
    pub hand: Hand,
    pub sequence: i32,
}

macro_rules! events {
    ($($name:ident)*) => {
        /// Inserts [`Events`] resources into the world for each client event.
        pub(crate) fn register_events(world: &mut World) {
            $(
                world.insert_resource(Events::<$name>::default());
            )*
        }

        /// Returns a system set for updating all the client events every tick.
        pub(crate) fn update_events_system_set() -> SystemSet {
            SystemSet::new()
            $(
                .with_system(Events::<$name>::update_system)
            )*
        }
    };
}

events! {
    QueryBlockEntity
    ChangeDifficulty
    MessageAcknowledgment
    ChatCommand
    ChatMessage
    ChatPreview
    PerformRespawn
    RequestStats
    UpdateSettings
    CommandSuggestionsRequest
    ClickContainerButton
    ClickContainer
    CloseContainer
    PluginMessage
    EditBook
    QueryEntity
    InteractWithEntity
    JigsawGenerate
    LockDifficulty
    SetPlayerPosition
    SetPlayerPositionAndRotation
    SetPlayerRotation
    SetPlayerOnGround
    MoveVehicle
    StartSneaking
    StopSneaking
    LeaveBed
    StartSprinting
    StopSprinting
    StartJumpWithHorse
    StopJumpWithHorse
    OpenHorseInventory
    StartFlyingWithElytra
    PaddleBoat
    PickItem
    PlaceRecipe
    StopFlying
    StartFlying
    StartDigging
    CancelDigging
    FinishDigging
    DropItem
    DropItemStack
    UpdateHeldItemState
    SwapItemInHand
    PlayerInput
    Pong
    PlayerSession
    ChangeRecipeBookSettings
    SetSeenRecipe
    RenameItem
    ResourcePackLoaded
    ResourcePackDeclined
    ResourcePackFailedDownload
    ResourcePackAccepted
    OpenAdvancementTab
    CloseAdvancementScreen
    SelectTrade
    SetBeaconEffect
    SetHeldItem
    ProgramCommandBlock
    ProgramCommandBlockMinecart
    SetCreativeModeSlot
    ProgramJigsawBlock
    ProgramStructureBlock
    UpdateSign
    SwingArm
    TeleportToEntity
    UseItemOnBlock
    UseItem
}
