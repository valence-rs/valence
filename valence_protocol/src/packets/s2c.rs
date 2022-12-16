use uuid::Uuid;
use valence_derive::{Decode, Encode, Packet};
use valence_nbt::Compound;

use crate::block_pos::BlockPos;
use crate::byte_angle::ByteAngle;
use crate::ident::Ident;
use crate::item::ItemStack;
use crate::raw_bytes::RawBytes;
use crate::text::Text;
use crate::types::{
    AttributeProperty, BossBarAction, ChunkDataBlockEntity, DeathLocation, Difficulty, GameMode,
    GameStateChangeReason, PlayerInfoAddPlayer, SignedProperty, SoundCategory,
    SyncPlayerPosLookFlags,
};
use crate::username::Username;
use crate::var_int::VarInt;
use crate::var_long::VarLong;
use crate::LengthPrefixedArray;

pub mod status {
    use super::*;

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x00]
    pub struct StatusResponse<'a> {
        pub json: &'a str,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x01]
    pub struct PingResponse {
        pub payload: u64,
    }

    packet_enum! {
        #[derive(Clone, Debug)]
        S2cStatusPacket<'a> {
            StatusResponse<'a>,
            PingResponse,
        }
    }
}

pub mod login {
    use super::*;

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x00]
    pub struct DisconnectLogin {
        pub reason: Text,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x01]
    pub struct EncryptionRequest<'a> {
        pub server_id: &'a str,
        pub public_key: &'a [u8],
        pub verify_token: &'a [u8],
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x02]
    pub struct LoginSuccess<'a> {
        pub uuid: Uuid,
        pub username: Username<&'a str>,
        pub properties: Vec<SignedProperty<'a>>,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x03]
    pub struct SetCompression {
        pub threshold: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x04]
    pub struct LoginPluginRequest<'a> {
        pub message_id: VarInt,
        pub channel: Ident<&'a str>,
        pub data: RawBytes<'a>,
    }

    packet_enum! {
        #[derive(Clone, Debug)]
        S2cLoginPacket<'a> {
            DisconnectLogin,
            EncryptionRequest<'a>,
            LoginSuccess<'a>,
            SetCompression,
            LoginPluginRequest<'a>,
        }
    }
}

pub mod play {
    use super::*;

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x00]
    pub struct SpawnEntity {
        pub entity_id: VarInt,
        pub object_uuid: Uuid,
        // TODO: EntityKind type?
        pub kind: VarInt,
        pub position: [f64; 3],
        pub pitch: ByteAngle,
        pub yaw: ByteAngle,
        pub head_yaw: ByteAngle,
        pub data: VarInt,
        pub velocity: [i16; 3],
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x01]
    pub struct SpawnExperienceOrb {
        pub entity_id: VarInt,
        pub position: [f64; 3],
        pub count: i16,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x02]
    pub struct SpawnPlayer {
        pub entity_id: VarInt,
        pub player_uuid: Uuid,
        pub position: [f64; 3],
        pub yaw: ByteAngle,
        pub pitch: ByteAngle,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x03]
    pub struct EntityAnimationS2c {
        pub entity_id: VarInt,
        pub animation: u8, // TODO: use Animation enum.
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x05]
    pub struct AcknowledgeBlockChange {
        pub sequence: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x06]
    pub struct SetBlockDestroyStage {
        pub entity_id: VarInt,
        pub position: BlockPos,
        pub destroy_stage: u8,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x07]
    pub struct BlockEntityData {
        pub position: BlockPos,
        // TODO: BlockEntityKind enum?
        pub kind: VarInt,
        pub data: Compound,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x09]
    pub struct BlockUpdate {
        pub position: BlockPos,
        pub block_id: VarInt,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x0a]
    pub struct BossBar {
        pub id: Uuid,
        pub action: BossBarAction,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x0b]
    pub struct SetDifficulty {
        pub difficulty: Difficulty,
        pub locked: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x0d]
    pub struct ClearTitles {
        pub reset: bool,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x11]
    pub struct SetContainerContent {
        pub window_id: u8,
        pub state_id: VarInt,
        pub slots: Vec<Option<ItemStack>>,
        pub carried_item: Option<ItemStack>,
    }

    #[derive(Copy, Clone, Debug, Encode, Packet)]
    #[packet_id = 0x11]
    pub struct SetContainerContentEncode<'a> {
        pub window_id: u8,
        pub state_id: VarInt,
        pub slots: &'a [Option<ItemStack>],
        pub carried_item: &'a Option<ItemStack>,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x12]
    pub struct SetContainerProperty {
        pub window_id: u8,
        pub property: i16,
        pub value: i16,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x13]
    pub struct SetContainerSlot {
        pub window_id: i8,
        pub state_id: VarInt,
        pub slot_idx: i16,
        pub slot_data: Option<ItemStack>,
    }

    #[derive(Clone, Debug, Encode, Packet)]
    #[packet_id = 0x13]
    pub struct SetContainerSlotEncode<'a> {
        pub window_id: i8,
        pub state_id: VarInt,
        pub slot_idx: i16,
        pub slot_data: Option<&'a ItemStack>,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x14]
    pub struct SetCooldown {
        pub item_id: VarInt,
        pub cooldown_ticks: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x16]
    pub struct PluginMessageS2c<'a> {
        pub channel: Ident<&'a str>,
        pub data: RawBytes<'a>,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x17]
    pub struct CustomSoundEffect<'a> {
        pub name: Ident<&'a str>,
        pub category: SoundCategory,
        pub position: [i32; 3],
        pub volume: f32,
        pub pitch: f32,
        pub seed: u64,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x19]
    pub struct DisconnectPlay {
        pub reason: Text,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x1a]
    pub struct EntityEvent {
        pub entity_id: i32,
        pub entity_status: u8,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x1c]
    pub struct UnloadChunk {
        pub chunk_x: i32,
        pub chunk_z: i32,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x1d]
    pub struct GameEvent {
        pub reason: GameStateChangeReason,
        pub value: f32,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x1f]
    pub struct WorldBorderInitialize {
        pub x: f64,
        pub z: f64,
        pub old_diameter: f64,
        pub new_diameter: f64,
        pub speed: VarLong,
        pub portal_teleport_boundary: VarInt,
        pub warning_blocks: VarInt,
        pub warning_time: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x20]
    pub struct KeepAliveS2c {
        pub id: u64,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x21]
    pub struct ChunkDataAndUpdateLight<'a> {
        pub chunk_x: i32,
        pub chunk_z: i32,
        pub heightmaps: Compound,
        pub blocks_and_biomes: &'a [u8],
        pub block_entities: Vec<ChunkDataBlockEntity>,
        pub trust_edges: bool,
        pub sky_light_mask: Vec<u64>,
        pub block_light_mask: Vec<u64>,
        pub empty_sky_light_mask: Vec<u64>,
        pub empty_block_light_mask: Vec<u64>,
        pub sky_light_arrays: Vec<LengthPrefixedArray<u8, 2048>>,
        pub block_light_arrays: Vec<LengthPrefixedArray<u8, 2048>>,
    }

    #[derive(Clone, Debug, Encode, Packet)]
    #[packet_id = 0x21]
    pub struct ChunkDataAndUpdateLightEncode<'a> {
        pub chunk_x: i32,
        pub chunk_z: i32,
        pub heightmaps: &'a Compound,
        pub blocks_and_biomes: &'a [u8],
        pub block_entities: &'a [ChunkDataBlockEntity],
        pub trust_edges: bool,
        pub sky_light_mask: &'a [u64],
        pub block_light_mask: &'a [u64],
        pub empty_sky_light_mask: &'a [u64],
        pub empty_block_light_mask: &'a [u64],
        pub sky_light_arrays: &'a [LengthPrefixedArray<u8, 2048>],
        pub block_light_arrays: &'a [LengthPrefixedArray<u8, 2048>],
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x23]
    pub struct ParticleS2c<'a> {
        pub particle_id: VarInt,
        pub long_distance: bool,
        pub position: [f64; 3],
        pub offset: [f32; 3],
        pub max_speed: f32,
        pub count: i32,
        pub data: RawBytes<'a>,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x25]
    pub struct LoginPlay<'a> {
        pub entity_id: i32,
        pub is_hardcore: bool,
        pub game_mode: GameMode,
        /// Same values as `game_mode` but with -1 to indicate no previous.
        pub previous_game_mode: i8,
        pub dimension_names: Vec<Ident<&'a str>>,
        pub registry_codec: Compound,
        pub dimension_type_name: Ident<&'a str>,
        pub dimension_name: Ident<&'a str>,
        pub hashed_seed: i64,
        pub max_players: VarInt,
        pub view_distance: VarInt,
        pub simulation_distance: VarInt,
        pub reduced_debug_info: bool,
        pub enable_respawn_screen: bool,
        pub is_debug: bool,
        pub is_flat: bool,
        pub last_death_location: Option<DeathLocation<'a>>,
    }

    // TODO: remove this.
    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x25]
    pub struct LoginPlayOwned {
        pub entity_id: i32,
        pub is_hardcore: bool,
        pub game_mode: GameMode,
        pub previous_game_mode: i8,
        pub dimension_names: Vec<Ident<String>>,
        pub registry_codec: Compound,
        pub dimension_type_name: Ident<String>,
        pub dimension_name: Ident<String>,
        pub hashed_seed: i64,
        pub max_players: VarInt,
        pub view_distance: VarInt,
        pub simulation_distance: VarInt,
        pub reduced_debug_info: bool,
        pub enable_respawn_screen: bool,
        pub is_debug: bool,
        pub is_flat: bool,
        pub last_death_location: Option<(Ident<String>, BlockPos)>,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x28]
    pub struct UpdateEntityPosition {
        pub entity_id: VarInt,
        pub delta: [i16; 3],
        pub on_ground: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x29]
    pub struct UpdateEntityPositionAndRotation {
        pub entity_id: VarInt,
        pub delta: [i16; 3],
        pub yaw: ByteAngle,
        pub pitch: ByteAngle,
        pub on_ground: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x2a]
    pub struct UpdateEntityRotation {
        pub entity_id: VarInt,
        pub yaw: ByteAngle,
        pub pitch: ByteAngle,
        pub on_ground: bool,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x2d]
    pub struct OpenScreen {
        pub window_id: VarInt,
        pub window_type: VarInt,
        pub window_title: Text,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x33]
    pub struct PlayerChatMessage<'a> {
        // TODO: A _lot_ of fields
        pub data: RawBytes<'a>,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x36]
    pub struct CombatDeath {
        pub player_id: VarInt,
        /// Killer's entity ID, -1 if no killer
        pub entity_id: i32,
        pub message: Text,
    }

    #[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x37]
    pub enum PlayerInfo<'a> {
        AddPlayer(Vec<PlayerInfoAddPlayer<'a>>),
        UpdateGameMode(Vec<(Uuid, GameMode)>),
        UpdateLatency(Vec<(Uuid, VarInt)>),
        UpdateDisplayName(Vec<(Uuid, Option<Text>)>),
        RemovePlayer(Vec<Uuid>),
    }

    #[derive(Copy, Clone, PartialEq, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x39]
    pub struct SynchronizePlayerPosition {
        pub position: [f64; 3],
        pub yaw: f32,
        pub pitch: f32,
        pub flags: SyncPlayerPosLookFlags,
        pub teleport_id: VarInt,
        pub dismount_vehicle: bool,
    }

    #[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x3b]
    pub struct RemoveEntities {
        pub entity_ids: Vec<VarInt>,
    }

    #[derive(Copy, Clone, PartialEq, Debug, Encode, Packet)]
    #[packet_id = 0x3b]
    pub struct RemoveEntitiesEncode<'a> {
        pub entity_ids: &'a [VarInt],
    }

    #[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x3d]
    pub struct ResourcePackS2c<'a> {
        pub url: &'a str,
        pub hash: &'a str,
        pub forced: bool,
        pub prompt_message: Option<Text>,
    }

    #[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x3e]
    pub struct Respawn<'a> {
        pub dimension_type_name: Ident<&'a str>,
        pub dimension_name: Ident<&'a str>,
        pub hashed_seed: u64,
        pub game_mode: GameMode,
        pub previous_game_mode: i8,
        pub is_debug: bool,
        pub is_flat: bool,
        pub copy_metadata: bool,
        pub last_death_location: Option<DeathLocation<'a>>,
    }

    // TODO: remove
    #[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x3e]
    pub struct RespawnOwned {
        pub dimension_type_name: Ident<String>,
        pub dimension_name: Ident<String>,
        pub hashed_seed: u64,
        pub game_mode: GameMode,
        pub previous_game_mode: i8,
        pub is_debug: bool,
        pub is_flat: bool,
        pub copy_metadata: bool,
        pub last_death_location: Option<(Ident<String>, BlockPos)>,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x3f]
    pub struct SetHeadRotation {
        pub entity_id: VarInt,
        pub head_yaw: ByteAngle,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x40]
    pub struct UpdateSectionBlocks {
        pub chunk_section_position: i64,
        pub invert_trust_edges: bool,
        pub blocks: Vec<VarLong>,
    }

    #[derive(Clone, Debug, Encode, Packet)]
    #[packet_id = 0x40]
    pub struct UpdateSectionBlocksEncode<'a> {
        pub chunk_section_position: i64,
        pub invert_trust_edges: bool,
        pub blocks: &'a [VarLong],
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x43]
    pub struct SetActionBarText(pub Text);

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x4a]
    pub struct SetHeldItemS2c {
        pub slot: u8,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x4b]
    pub struct SetCenterChunk {
        pub chunk_x: VarInt,
        pub chunk_z: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x4c]
    pub struct SetRenderDistance(pub VarInt);

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x4d]
    pub struct SetDefaultSpawnPosition {
        pub position: BlockPos,
        pub angle: f32,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x50]
    pub struct SetEntityMetadata<'a> {
        pub entity_id: VarInt,
        pub metadata: RawBytes<'a>,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x52]
    pub struct SetEntityVelocity {
        pub entity_id: VarInt,
        pub velocity: [i16; 3],
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x54]
    pub struct SetExperience {
        pub bar: f32,
        pub level: VarInt,
        pub total_xp: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x55]
    pub struct SetHealth {
        pub health: f32,
        pub food: VarInt,
        pub food_saturation: f32,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x5b]
    pub struct SetSubtitleText(pub Text);

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x5c]
    pub struct UpdateTime {
        /// The age of the world in 1/20ths of a second.
        pub world_age: i64,
        /// The current time of day in 1/20ths of a second.
        /// The value should be in the range \[0, 24000].
        /// 6000 is noon, 12000 is sunset, and 18000 is midnight.
        pub time_of_day: i64,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x5d]
    pub struct SetTitleText(pub Text);

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x5e]
    pub struct SetTitleAnimationTimes {
        /// Ticks to spend fading in.
        pub fade_in: i32,
        /// Ticks to keep the title displayed.
        pub stay: i32,
        /// Ticks to spend fading out.
        pub fade_out: i32,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x5f]
    pub struct EntitySoundEffect {
        pub id: VarInt,
        pub category: SoundCategory,
        pub entity_id: VarInt,
        pub volume: f32,
        pub pitch: f32,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x60]
    pub struct SoundEffect {
        pub id: VarInt,
        pub category: SoundCategory,
        pub position: [i32; 3],
        pub volume: f32,
        pub pitch: f32,
        pub seed: i64,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x62]
    pub struct SystemChatMessage {
        pub chat: Text,
        /// Index into the chat type registry.
        pub kind: VarInt,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x63]
    pub struct SetTabListHeaderAndFooter {
        pub header: Text,
        pub footer: Text,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x66]
    pub struct TeleportEntity {
        pub entity_id: VarInt,
        pub position: [f64; 3],
        pub yaw: ByteAngle,
        pub pitch: ByteAngle,
        pub on_ground: bool,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x68]
    pub struct UpdateAttributes<'a> {
        pub entity_id: VarInt,
        pub properties: Vec<AttributeProperty<'a>>,
    }

    packet_enum! {
        #[derive(Clone, Debug)]
        S2cPlayPacket<'a> {
            SpawnEntity,
            SpawnExperienceOrb,
            SpawnPlayer,
            EntityAnimationS2c,
            AcknowledgeBlockChange,
            SetBlockDestroyStage,
            BlockEntityData,
            BlockUpdate,
            BossBar,
            SetDifficulty,
            ClearTitles,
            SetContainerContent,
            SetContainerProperty,
            SetContainerSlot,
            SetCooldown,
            PluginMessageS2c<'a>,
            CustomSoundEffect<'a>,
            DisconnectPlay,
            EntityEvent,
            UnloadChunk,
            GameEvent,
            WorldBorderInitialize,
            KeepAliveS2c,
            ChunkDataAndUpdateLight<'a>,
            ParticleS2c<'a>,
            LoginPlay<'a>,
            UpdateEntityPosition,
            UpdateEntityPositionAndRotation,
            UpdateEntityRotation,
            OpenScreen,
            PlayerChatMessage<'a>,
            CombatDeath,
            PlayerInfo<'a>,
            SynchronizePlayerPosition,
            RemoveEntities,
            ResourcePackS2c<'a>,
            Respawn<'a>,
            SetHeadRotation,
            UpdateSectionBlocks,
            SetActionBarText,
            SetHeldItemS2c,
            SetCenterChunk,
            SetRenderDistance,
            SetDefaultSpawnPosition,
            SetEntityMetadata<'a>,
            SetEntityVelocity,
            SetExperience,
            SetHealth,
            SetSubtitleText,
            UpdateTime,
            SetTitleText,
            SetTitleAnimationTimes,
            EntitySoundEffect,
            SoundEffect,
            SystemChatMessage,
            SetTabListHeaderAndFooter,
            TeleportEntity,
            UpdateAttributes<'a>,
        }
    }
}
