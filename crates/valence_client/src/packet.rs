//! Common packets for this crate.

use std::borrow::Cow;

use bitfield_struct::bitfield;
use glam::DVec3;
use uuid::Uuid;
use valence_core::block_pos::BlockPos;
use valence_core::difficulty::Difficulty;
use valence_core::direction::Direction;
use valence_core::game_mode::GameMode;
use valence_core::hand::Hand;
use valence_core::ident::Ident;
use valence_core::protocol::byte_angle::ByteAngle;
use valence_core::protocol::global_pos::GlobalPos;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::var_long::VarLong;
use valence_core::protocol::{packet_id, Decode, Encode, Packet};
use valence_core::text::Text;
use valence_nbt::Compound;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::BUNDLE_SPLITTER)]
pub struct BundleSplitterS2c;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::BOAT_PADDLE_STATE_C2S)]
pub struct BoatPaddleStateC2s {
    pub left_paddle_turning: bool,
    pub right_paddle_turning: bool,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::BOOK_UPDATE_C2S)]
pub struct BookUpdateC2s<'a> {
    pub slot: VarInt,
    pub entries: Vec<&'a str>,
    pub title: Option<&'a str>,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::JIGSAW_GENERATING_C2S)]
pub struct JigsawGeneratingC2s {
    pub position: BlockPos,
    pub levels: VarInt,
    pub keep_jigsaws: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAY_PONG_C2S)]
pub struct PlayPongC2s {
    pub id: i32,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_ACTION_C2S)]
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

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_INPUT_C2S)]
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

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::QUERY_BLOCK_NBT_C2S)]
pub struct QueryBlockNbtC2s {
    pub transaction_id: VarInt,
    pub position: BlockPos,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::QUERY_ENTITY_NBT_C2S)]
pub struct QueryEntityNbtC2s {
    pub transaction_id: VarInt,
    pub entity_id: VarInt,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SPECTATOR_TELEPORT_C2S)]
pub struct SpectatorTeleportC2s {
    pub target: Uuid,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UPDATE_COMMAND_BLOCK_MINECART_C2S)]
pub struct UpdateCommandBlockMinecartC2s<'a> {
    pub entity_id: VarInt,
    pub command: &'a str,
    pub track_output: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UPDATE_COMMAND_BLOCK_C2S)]
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

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UPDATE_DIFFICULTY_LOCK_C2S)]
pub struct UpdateDifficultyLockC2s {
    pub locked: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UPDATE_DIFFICULTY_C2S)]
pub struct UpdateDifficultyC2s {
    pub difficulty: Difficulty,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UPDATE_JIGSAW_C2S)]
pub struct UpdateJigsawC2s<'a> {
    pub position: BlockPos,
    pub name: Ident<Cow<'a, str>>,
    pub target: Ident<Cow<'a, str>>,
    pub pool: Ident<Cow<'a, str>>,
    pub final_state: &'a str,
    pub joint_type: &'a str,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UPDATE_PLAYER_ABILITIES_C2S)]
pub enum UpdatePlayerAbilitiesC2s {
    #[packet(tag = 0b00)]
    StopFlying,
    #[packet(tag = 0b10)]
    StartFlying,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UPDATE_SIGN_C2S)]
pub struct UpdateSignC2s<'a> {
    pub position: BlockPos,
    pub is_front_text: bool,
    pub lines: [&'a str; 4],
}

pub mod structure_block {

    use super::*;

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::UPDATE_STRUCTURE_BLOCK_C2S)]
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

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::DEATH_MESSAGE_S2C)]
pub struct DeathMessageS2c<'a> {
    pub player_id: VarInt,
    pub message: Cow<'a, Text>,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::DAMAGE_TILT_S2C)]
pub struct DamageTiltS2c {
    /// The ID of the entity taking damage.
    pub entity_id: VarInt,
    /// The direction the damage is coming from in relation to the entity.
    pub yaw: f32,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::DIFFICULTY_S2C)]
pub struct DifficultyS2c {
    pub difficulty: Difficulty,
    pub locked: bool,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::DISCONNECT_S2C)]
pub struct DisconnectS2c<'a> {
    pub reason: Cow<'a, Text>,
}

/// Unused by notchian clients.
#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTER_COMBAT_S2C)]
pub struct EnterCombatS2c;

/// Unused by notchian clients.
#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::END_COMBAT_S2C)]
pub struct EndCombatS2c {
    pub duration: VarInt,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::EXPERIENCE_BAR_UPDATE_S2C)]
pub struct ExperienceBarUpdateS2c {
    pub bar: f32,
    pub level: VarInt,
    pub total_xp: VarInt,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::FEATURES_S2C)]
pub struct FeaturesS2c<'a> {
    pub features: Vec<Ident<Cow<'a, str>>>,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::GAME_JOIN_S2C)]
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
    pub portal_cooldown: VarInt,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::GAME_STATE_CHANGE_S2C)]
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

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::HEALTH_UPDATE_S2C)]
pub struct HealthUpdateS2c {
    pub health: f32,
    pub food: VarInt,
    pub food_saturation: f32,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_ABILITIES_S2C)]
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

#[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_RESPAWN_S2C)]
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
    pub portal_cooldown: VarInt,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_SPAWN_POSITION_S2C)]
pub struct PlayerSpawnPositionS2c {
    pub position: BlockPos,
    pub angle: f32,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SERVER_METADATA_S2C)]
pub struct ServerMetadataS2c<'a> {
    pub motd: Cow<'a, Text>,
    pub icon: Option<&'a [u8]>,
    pub enforce_secure_chat: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SIGN_EDITOR_OPEN_S2C)]
pub struct SignEditorOpenS2c {
    pub location: BlockPos,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SIMULATION_DISTANCE_S2C)]
pub struct SimulationDistanceS2c {
    pub simulation_distance: VarInt,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::STATISTICS_S2C)]
pub struct StatisticsS2c {
    pub statistics: Vec<Statistic>,
}

#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode)]
pub struct Statistic {
    pub category_id: VarInt,
    pub statistic_id: VarInt,
    pub value: VarInt,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::VEHICLE_MOVE_S2C)]
pub struct VehicleMoveS2c {
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::OPEN_WRITTEN_BOOK_S2C)]
pub struct OpenWrittenBookS2c {
    pub hand: Hand,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAY_PING_S2C)]
pub struct PlayPingS2c {
    pub id: i32,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::NBT_QUERY_RESPONSE_S2C)]
pub struct NbtQueryResponseS2c {
    pub transaction_id: VarInt,
    pub nbt: Compound,
}
