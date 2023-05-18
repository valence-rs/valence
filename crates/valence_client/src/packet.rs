use std::borrow::Cow;

use valence_core::block_pos::BlockPos;
use valence_core::direction::Direction;
use valence_core::game_mode::GameMode;
use valence_core::hand::Hand;
use valence_core::ident::Ident;
use valence_core::protocol::global_pos::GlobalPos;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct BundleSplitter;

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct BoatPaddleStateC2s {
    pub left_paddle_turning: bool,
    pub right_paddle_turning: bool,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct BookUpdateC2s<'a> {
    pub slot: VarInt,
    pub entries: Vec<&'a str>,
    pub title: Option<&'a str>,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ClientCommandC2s {
    pub entity_id: VarInt,
    pub action: ClientCommand,
    pub jump_boost: VarInt,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum ClientCommand {
    StartSneaking,
    StopSneaking,
    LeaveBed,
    StartSprinting,
    StopSprinting,
    StartJumpWithHorse,
    StopJumpWithHorse,
    OpenHorseInventory,
    StartFlyingWithElytra,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ClientSettingsC2s<'a> {
    pub locale: &'a str,
    pub view_distance: u8,
    pub chat_mode: ChatMode,
    pub chat_colors: bool,
    pub displayed_skin_parts: DisplayedSkinParts,
    pub main_arm: MainArm,
    pub enable_text_filtering: bool,
    pub allow_server_listings: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Default, Debug, Encode, Decode)]
pub enum ChatMode {
    Enabled,
    CommandsOnly,
    #[default]
    Hidden,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct DisplayedSkinParts {
    pub cape: bool,
    pub jacket: bool,
    pub left_sleeve: bool,
    pub right_sleeve: bool,
    pub left_pants_leg: bool,
    pub right_pants_leg: bool,
    pub hat: bool,
    _pad: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Encode, Decode)]
pub enum MainArm {
    Left,
    #[default]
    Right,
}

#[derive(Clone, Debug, Encode, Decode)]
pub enum ClientStatusC2s {
    PerformRespawn,
    RequestStats,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct CustomPayloadC2s<'a> {
    pub channel: Ident<Cow<'a, str>>,
    pub data: RawBytes<'a>,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct HandSwingC2s {
    pub hand: Hand,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct JigsawGeneratingC2s {
    pub position: BlockPos,
    pub levels: VarInt,
    pub keep_jigsaws: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct KeepAliveC2s {
    pub id: u64,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlayPongC2s {
    pub id: i32,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlayerActionC2s {
    pub action: PlayerAction,
    pub position: BlockPos,
    pub direction: Direction,
    pub sequence: VarInt,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum PlayerAction {
    StartDestroyBlock,
    AbortDestroyBlock,
    StopDestroyBlock,
    DropAllItems,
    DropItem,
    ReleaseUseItem,
    SwapItemWithOffhand,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlayerInputC2s {
    pub sideways: f32,
    pub forward: f32,
    pub flags: PlayerInputFlags,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct PlayerInputFlags {
    pub jump: bool,
    pub unmount: bool,
    #[bits(6)]
    _pad: u8,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlayerInteractBlockC2s {
    pub hand: Hand,
    pub position: BlockPos,
    pub face: Direction,
    pub cursor_pos: Vec3,
    pub head_inside_block: bool,
    pub sequence: VarInt,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlayerInteractEntityC2s {
    pub entity_id: VarInt,
    pub interact: EntityInteraction,
    pub sneaking: bool,
}

#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode)]
pub enum EntityInteraction {
    Interact(Hand),
    Attack,
    InteractAt { target: Vec3, hand: Hand },
}

pub mod movement {
    use super::*;

    #[derive(Copy, Clone, Debug, Encode, Decode)]
    pub struct PositionAndOnGround {
        pub position: DVec3,
        pub on_ground: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode)]
    pub struct Full {
        pub position: DVec3,
        pub yaw: f32,
        pub pitch: f32,
        pub on_ground: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode)]
    pub struct LookAndOnGround {
        pub yaw: f32,
        pub pitch: f32,
        pub on_ground: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode)]
    pub struct OnGroundOnly {
        pub on_ground: bool,
    }
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct QueryBlockNbtC2s {
    pub transaction_id: VarInt,
    pub position: BlockPos,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct QueryEntityNbtC2s {
    pub transaction_id: VarInt,
    pub entity_id: VarInt,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub enum ResourcePackStatusC2s {
    SuccessfullyLoaded,
    Declined,
    FailedDownload,
    Accepted,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct SpectatorTeleportC2s {
    pub target: Uuid,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct TeleportConfirmC2s {
    pub teleport_id: VarInt,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct VehicleMoveC2s {
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct UpdateCommandBlockMinecartC2s<'a> {
    pub entity_id: VarInt,
    pub command: &'a str,
    pub track_output: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct UpdateCommandBlockC2s<'a> {
    pub position: BlockPos,
    pub command: &'a str,
    pub mode: UpdateCommandBlockMode,
    pub flags: UpdateCommandBlockFlags,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum UpdateCommandBlockMode {
    Sequence,
    Auto,
    Redstone,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct UpdateCommandBlockFlags {
    pub track_output: bool,
    pub conditional: bool,
    pub automatic: bool,
    #[bits(5)]
    _pad: u8,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct UpdateDifficultyLockC2s {
    pub locked: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub struct UpdateDifficultyC2s {
    pub difficulty: Difficulty,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct UpdateJigsawC2s<'a> {
    pub position: BlockPos,
    pub name: Ident<Cow<'a, str>>,
    pub target: Ident<Cow<'a, str>>,
    pub pool: Ident<Cow<'a, str>>,
    pub final_state: &'a str,
    pub joint_type: &'a str,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub enum UpdatePlayerAbilitiesC2s {
    #[packet(tag = 0b00)]
    StopFlying,
    #[packet(tag = 0b10)]
    StartFlying,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct UpdateSignC2s<'a> {
    pub position: BlockPos,
    pub lines: [&'a str; 4],
}

pub mod structure_block {
    #[derive(Copy, Clone, Debug, Encode, Decode)]
    pub struct UpdateStructureBlockC2s<'a> {
        pub position: BlockPos,
        pub action: Action,
        pub mode: Mode,
        pub name: &'a str,
        pub offset_xyz: [i8; 3],
        pub size_xyz: [i8; 3],
        pub mirror: Mirror,
        pub rotation: Rotation,
        pub metadata: &'a str,
        pub integrity: f32,
        pub seed: VarLong,
        pub flags: Flags,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
    pub enum Action {
        UpdateData,
        SaveStructure,
        LoadStructure,
        DetectSize,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
    pub enum Mode {
        Save,
        Load,
        Corner,
        Data,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
    pub enum Mirror {
        None,
        LeftRight,
        FrontBack,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
    pub enum Rotation {
        None,
        Clockwise90,
        Clockwise180,
        Counterclockwise90,
    }

    #[bitfield(u8)]
    #[derive(PartialEq, Eq, Encode, Decode)]
    pub struct Flags {
        pub ignore_entities: bool,
        pub show_air: bool,
        pub show_bounding_box: bool,
        #[bits(5)]
        _pad: u8,
    }
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct KeepAliveS2c {
    pub id: u64,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct DeathMessageS2c<'a> {
    pub player_id: VarInt,
    /// Killer's entity ID, -1 if no killer
    pub entity_id: i32,
    pub message: Cow<'a, Text>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct CustomPayloadS2c<'a> {
    pub channel: Ident<Cow<'a, str>>,
    pub data: RawBytes<'a>,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ClearTitleS2c {
    pub reset: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct DamageTiltS2c {
    /// The ID of the entity taking damage.
    pub entity_id: VarInt,
    /// The direction the damage is coming from in relation to the entity.
    pub yaw: f32,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct DifficultyS2c {
    pub difficulty: Difficulty,
    pub locked: bool,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct DisconnectS2c<'a> {
    pub reason: Cow<'a, Text>,
}

/// Unused by notchian clients.
#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode)]
pub struct EnterCombatS2c;

/// Unused by notchian clients.
#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode)]
pub struct EndCombatS2c {
    pub duration: VarInt,
    pub entity_id: i32,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ExperienceBarUpdateS2c {
    pub bar: f32,
    pub level: VarInt,
    pub total_xp: VarInt,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct FeaturesS2c<'a> {
    pub features: Vec<Ident<Cow<'a, str>>>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct GameJoinS2c<'a> {
    pub entity_id: i32,
    pub is_hardcore: bool,
    pub game_mode: GameMode,
    /// Same values as `game_mode` but with -1 to indicate no previous.
    pub previous_game_mode: i8,
    pub dimension_names: Vec<Ident<Cow<'a, str>>>,
    pub registry_codec: Cow<'a, Compound>,
    pub dimension_type_name: Ident<Cow<'a, str>>,
    pub dimension_name: Ident<Cow<'a, str>>,
    pub hashed_seed: i64,
    pub max_players: VarInt,
    pub view_distance: VarInt,
    pub simulation_distance: VarInt,
    pub reduced_debug_info: bool,
    pub enable_respawn_screen: bool,
    pub is_debug: bool,
    pub is_flat: bool,
    pub last_death_location: Option<GlobalPos<'a>>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct GameMessageS2c<'a> {
    pub chat: Cow<'a, Text>,
    /// Whether the message is in the actionbar or the chat.
    pub overlay: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct GameStateChangeS2c {
    pub kind: GameEventKind,
    pub value: f32,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum GameEventKind {
    NoRespawnBlockAvailable,
    EndRaining,
    BeginRaining,
    ChangeGameMode,
    WinGame,
    DemoEvent,
    ArrowHitPlayer,
    RainLevelChange,
    ThunderLevelChange,
    PlayPufferfishStingSound,
    PlayElderGuardianMobAppearance,
    EnableRespawnScreen,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct HealthUpdateS2c {
    pub health: f32,
    pub food: VarInt,
    pub food_saturation: f32,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct PlayerAbilitiesS2c {
    pub flags: PlayerAbilitiesFlags,
    pub flying_speed: f32,
    pub fov_modifier: f32,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct PlayerAbilitiesFlags {
    pub invulnerable: bool,
    pub flying: bool,
    pub allow_flying: bool,
    pub instant_break: bool,
    #[bits(4)]
    _pad: u8,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlayerActionResponseS2c {
    pub sequence: VarInt,
}

#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode)]
pub struct PlayerPositionLookS2c {
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub flags: PlayerPositionLookFlags,
    pub teleport_id: VarInt,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct PlayerPositionLookFlags {
    pub x: bool,
    pub y: bool,
    pub z: bool,
    pub y_rot: bool,
    pub x_rot: bool,
    #[bits(3)]
    _pad: u8,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct PlayerRespawnS2c<'a> {
    pub dimension_type_name: Ident<Cow<'a, str>>,
    pub dimension_name: Ident<Cow<'a, str>>,
    pub hashed_seed: u64,
    pub game_mode: GameMode,
    pub previous_game_mode: i8,
    pub is_debug: bool,
    pub is_flat: bool,
    pub copy_metadata: bool,
    pub last_death_location: Option<GlobalPos<'a>>,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlayerSpawnPositionS2c {
    pub position: BlockPos,
    pub angle: f32,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlayerSpawnS2c {
    pub entity_id: VarInt,
    pub player_uuid: Uuid,
    pub position: DVec3,
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct ResourcePackSendS2c<'a> {
    pub url: &'a str,
    pub hash: &'a str,
    pub forced: bool,
    pub prompt_message: Option<Cow<'a, Text>>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct ServerMetadataS2c<'a> {
    pub motd: Cow<'a, Text>,
    pub icon: Option<&'a [u8]>,
    pub enforce_secure_chat: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct SignEditorOpenS2c {
    pub location: BlockPos,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct SimulationDistanceS2c {
    pub simulation_distance: VarInt,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct StatisticsS2c {
    pub statistics: Vec<Statistic>,
}

#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode)]
pub struct Statistic {
    pub category_id: VarInt,
    pub statistic_id: VarInt,
    pub value: VarInt,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct VehicleMoveS2c {
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct OpenWrittenBookS2c {
    pub hand: Hand,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct OverlayMessageS2c<'a> {
    pub action_bar_text: Cow<'a, Text>,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlayPingS2c {
    pub id: i32,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct TitleS2c<'a> {
    pub title_text: Cow<'a, Text>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct SubtitleS2c<'a> {
    pub subtitle_text: Cow<'a, Text>,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct TitleFadeS2c {
    /// Ticks to spend fading in.
    pub fade_in: i32,
    /// Ticks to keep the title displayed.
    pub stay: i32,
    /// Ticks to spend fading out.
    pub fade_out: i32,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct NbtQueryResponseS2c {
    pub transaction_id: VarInt,
    pub nbt: Compound,
}

