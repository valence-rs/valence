use std::borrow::Cow;

use uuid::Uuid;
use valence_nbt::Compound;

use crate::block::BlockEntityKind;
use crate::block_pos::BlockPos;
use crate::byte_angle::ByteAngle;
use crate::ident::Ident;
use crate::item::ItemStack;
use crate::raw_bytes::RawBytes;
use crate::text::Text;
use crate::types::{
    AttributeProperty, BossBarAction, ChatSuggestionAction, ChunkDataBlockEntity,
    CommandSuggestionMatch, Difficulty, EntityEffectFlags, FeetOrEyes, GameEventKind, GameMode,
    GlobalPos, Hand, LookAtEntity, MerchantTrade, PlayerAbilitiesFlags, Property, SoundCategory,
    Statistic, SyncPlayerPosLookFlags, TagGroup, UpdateObjectiveMode, UpdateScoreAction,
    WindowType,
};
use crate::username::Username;
use crate::var_int::VarInt;
use crate::var_long::VarLong;
use crate::{Decode, DecodePacket, Encode, EncodePacket, LengthPrefixedArray};

pub mod commands;
pub mod declare_recipes;
pub mod map_update;
pub mod message_signature;
pub mod particle;
pub mod chat_message;
pub mod player_list;
pub mod entity_equipment_update;
pub mod sound_id;
pub mod stop_sound;
pub mod advancement_update;
pub mod unlock_recipes;
pub mod update_teams;

pub mod status {
    use super::*;

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x00]
    pub struct QueryResponseS2c<'a> {
        pub json: &'a str,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x01]
    pub struct QueryPongS2c {
        pub payload: u64,
    }

    packet_enum! {
        #[derive(Clone)]
        S2cStatusPacket<'a> {
            QueryResponseS2c<'a>,
            QueryPongS2c,
        }
    }
}

pub mod login {
    use super::*;

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x00]
    pub struct LoginDisconnectS2c<'a> {
        pub reason: Cow<'a, Text>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x01]
    pub struct LoginHelloS2c<'a> {
        pub server_id: &'a str,
        pub public_key: &'a [u8],
        pub verify_token: &'a [u8],
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x02]
    pub struct LoginSuccessS2c<'a> {
        pub uuid: Uuid,
        pub username: Username<&'a str>,
        pub properties: Cow<'a, [Property]>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x03]
    pub struct LoginCompressionS2c {
        pub threshold: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x04]
    pub struct LoginQueryRequestS2c<'a> {
        pub message_id: VarInt,
        pub channel: Ident<&'a str>,
        pub data: RawBytes<'a>,
    }

    packet_enum! {
        #[derive(Clone)]
        S2cLoginPacket<'a> {
            LoginDisconnectS2c<'a>,
            LoginHelloS2c<'a>,
            LoginSuccessS2c<'a>,
            LoginCompressionS2c,
            LoginQueryRequestS2c<'a>,
        }
    }
}

pub mod play {
    use commands::Node;
    pub use map_update::MapUpdateS2c;
    pub use message_signature::MessageSignature;
    pub use particle::ParticleS2c;
    pub use chat_message::ChatMessageS2c;
    pub use player_list::PlayerListS2c;
    pub use entity_equipment_update::EntityEquipmentUpdateS2c;
    pub use sound_id::SoundId;
    pub use stop_sound::StopSoundS2c;
    pub use advancement_update::AdvancementUpdateS2c;
    pub use unlock_recipes::UnlockRecipesS2c;

    use super::*;
    use crate::packet::s2c::declare_recipes::DeclaredRecipe;
    use crate::packet::s2c::update_teams::UpdateTeamsMode;

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x00]
    pub struct EntitySpawnS2c {
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

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x01]
    pub struct ExperienceOrbSpawnS2c {
        pub entity_id: VarInt,
        pub position: [f64; 3],
        pub count: i16,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x02]
    pub struct PlayerSpawnS2c {
        pub entity_id: VarInt,
        pub player_uuid: Uuid,
        pub position: [f64; 3],
        pub yaw: ByteAngle,
        pub pitch: ByteAngle,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x03]
    pub struct EntityAnimationS2c {
        pub entity_id: VarInt,
        pub animation: u8, // TODO: use Animation enum.
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x04]
    pub struct StatisticsS2c {
        pub statistics: Vec<Statistic>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x05]
    pub struct PlayerActionResponseS2c {
        pub sequence: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x06]
    pub struct BlockBreakingProgressS2c {
        pub entity_id: VarInt,
        pub position: BlockPos,
        pub destroy_stage: u8,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x07]
    pub struct BlockEntityUpdateS2c<'a> {
        pub position: BlockPos,
        pub kind: BlockEntityKind,
        pub data: Cow<'a, Compound>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x08]
    pub struct BlockEventS2c {
        pub position: BlockPos,
        pub action_id: u8,
        pub action_parameter: u8,
        pub block_type: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x09]
    pub struct BlockUpdateS2c {
        pub position: BlockPos,
        pub block_id: VarInt,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x0a]
    pub struct BossBarS2c {
        pub id: Uuid,
        pub action: BossBarAction,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x0b]
    pub struct DifficultyS2c {
        pub difficulty: Difficulty,
        pub locked: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x0c]
    pub struct ClearTitlesS2c {
        pub reset: bool,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x0d]
    pub struct CommandSuggestionsS2c<'a> {
        pub id: VarInt,
        pub start: VarInt,
        pub length: VarInt,
        pub matches: Vec<CommandSuggestionMatch<'a>>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x0e]
    pub struct CommandTreeS2c<'a> {
        pub commands: Vec<Node<'a>>,
        pub root_index: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x0f]
    pub struct CloseScreenS2c {
        /// Ignored by notchian clients.
        pub window_id: u8,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x10]
    pub struct InventoryS2c<'a> {
        pub window_id: u8,
        pub state_id: VarInt,
        pub slots: Cow<'a, [Option<ItemStack>]>,
        pub carried_item: Cow<'a, Option<ItemStack>>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x11]
    pub struct ScreenHandlerPropertyUpdateS2c {
        pub window_id: u8,
        pub property: i16,
        pub value: i16,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x12]
    pub struct ScreenHandlerSlotUpdateS2c<'a> {
        pub window_id: i8,
        pub state_id: VarInt,
        pub slot_idx: i16,
        pub slot_data: Cow<'a, Option<ItemStack>>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x13]
    pub struct CooldownUpdateS2c {
        pub item_id: VarInt,
        pub cooldown_ticks: VarInt,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x14]
    pub struct ChatSuggestionsS2c<'a> {
        pub action: ChatSuggestionAction,
        pub entries: Vec<&'a str>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x15]
    pub struct CustomPayloadS2c<'a> {
        pub channel: Ident<&'a str>,
        pub data: RawBytes<'a>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x16]
    pub struct RemoveMessageS2c<'a> {
        pub signature: MessageSignature<'a>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x17]
    pub struct DisconnectS2c<'a> {
        pub reason: Cow<'a, Text>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x18]
    pub struct ProfilelessChatMessageS2c<'a> {
        pub message: Cow<'a, Text>,
        pub chat_type: VarInt,
        pub chat_type_name: Cow<'a, Text>,
        pub target_name: Option<Cow<'a, Text>>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x19]
    pub struct EntityStatusS2c {
        pub entity_id: i32,
        pub entity_status: u8,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x1a]
    pub struct ExplosionS2c<'a> {
        pub window_id: u8,
        pub recipe: Ident<&'a str>,
        pub make_all: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x1b]
    pub struct UnloadChunkS2c {
        pub chunk_x: i32,
        pub chunk_z: i32,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x1c]
    pub struct GameStateChangeS2c {
        pub kind: GameEventKind,
        pub value: f32,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x1d]
    pub struct OpenHorseScreenS2c {
        pub window_id: u8,
        pub slot_count: VarInt,
        pub entity_id: i32,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x1e]
    pub struct WorldBorderInitializeS2c {
        pub x: f64,
        pub z: f64,
        pub old_diameter: f64,
        pub new_diameter: f64,
        pub speed: VarLong,
        pub portal_teleport_boundary: VarInt,
        pub warning_blocks: VarInt,
        pub warning_time: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x1f]
    pub struct KeepAliveS2c {
        pub id: u64,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x20]
    pub struct ChunkDataS2c<'a> {
        pub chunk_x: i32,
        pub chunk_z: i32,
        pub heightmaps: Cow<'a, Compound>,
        pub blocks_and_biomes: &'a [u8],
        pub block_entities: Cow<'a, [ChunkDataBlockEntity<'a>]>,
        pub trust_edges: bool,
        pub sky_light_mask: Cow<'a, [u64]>,
        pub block_light_mask: Cow<'a, [u64]>,
        pub empty_sky_light_mask: Cow<'a, [u64]>,
        pub empty_block_light_mask: Cow<'a, [u64]>,
        pub sky_light_arrays: Cow<'a, [LengthPrefixedArray<u8, 2048>]>,
        pub block_light_arrays: Cow<'a, [LengthPrefixedArray<u8, 2048>]>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x21]
    pub struct WorldEventS2c {
        pub event: i32,
        pub location: BlockPos,
        pub data: i32,
        pub disable_relative_volume: bool,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x23]
    pub struct LightUpdateS2c {
        pub chunk_x: VarInt,
        pub chunk_z: VarInt,
        pub trust_edges: bool,
        pub sky_light_mask: Vec<u64>,
        pub block_light_mask: Vec<u64>,
        pub empty_sky_light_mask: Vec<u64>,
        pub empty_block_light_mask: Vec<u64>,
        pub sky_light_arrays: Vec<LengthPrefixedArray<u8, 2048>>,
        pub block_light_arrays: Vec<LengthPrefixedArray<u8, 2048>>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x24]
    pub struct GameJoinS2c<'a> {
        pub entity_id: i32,
        pub is_hardcore: bool,
        pub game_mode: GameMode,
        /// Same values as `game_mode` but with -1 to indicate no previous.
        pub previous_game_mode: i8,
        pub dimension_names: Vec<Ident<&'a str>>,
        pub registry_codec: Cow<'a, Compound>,
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
        pub last_death_location: Option<GlobalPos<'a>>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x26]
    pub struct SetTradeOffersS2c {
        pub window_id: VarInt,
        pub trades: Vec<MerchantTrade>,
        pub villager_level: VarInt,
        pub experience: VarInt,
        pub is_regular_villager: bool,
        pub can_restock: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x27]
    pub struct MoveRelativeS2c {
        pub entity_id: VarInt,
        pub delta: [i16; 3],
        pub on_ground: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x28]
    pub struct RotateAndMoveRelativeS2c {
        pub entity_id: VarInt,
        pub delta: [i16; 3],
        pub yaw: ByteAngle,
        pub pitch: ByteAngle,
        pub on_ground: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x29]
    pub struct RotateS2c {
        pub entity_id: VarInt,
        pub yaw: ByteAngle,
        pub pitch: ByteAngle,
        pub on_ground: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x2a]
    pub struct VehicleMoveS2c {
        pub position: [f64; 3],
        pub yaw: f32,
        pub pitch: f32,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x2b]
    pub struct OpenWrittenBookS2c {
        pub hand: Hand,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x2c]
    pub struct OpenScreenS2c<'a> {
        pub window_id: VarInt,
        pub window_type: WindowType,
        pub window_title: Cow<'a, Text>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x2d]
    pub struct SignEditorOpen {
        pub location: BlockPos,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x2e]
    pub struct PlayPingS2c {
        pub id: i32,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x2f]
    pub struct CraftFailedResponseS2c<'a> {
        pub window_id: u8,
        pub recipe: Ident<&'a str>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x30]
    pub struct PlayerAbilitiesS2c {
        pub flags: PlayerAbilitiesFlags,
        pub flying_speed: f32,
        pub fov_modifier: f32,
    }

    /// Unused by notchian clients.
    #[derive(Copy, Clone, PartialEq, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x32]
    pub struct EndCombatS2c {
        pub duration: VarInt,
        pub entity_id: i32,
    }

    /// Unused by notchian clients.
    #[derive(Copy, Clone, PartialEq, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x33]
    pub struct EnterCombatS2c;

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x34]
    pub struct DeathMessageS2c<'a> {
        pub player_id: VarInt,
        /// Killer's entity ID, -1 if no killer
        pub entity_id: i32,
        pub message: Cow<'a, Text>,
    }

    #[derive(Clone, PartialEq, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x35]
    pub struct PlayerRemove<'a> {
        pub uuids: Cow<'a, [Uuid]>,
    }

    #[derive(Copy, Clone, PartialEq, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x37]
    pub struct LookAtS2c {
        pub feet_eyes: FeetOrEyes,
        pub target_position: [f64; 3],
        pub entity_to_face: Option<LookAtEntity>,
    }

    #[derive(Copy, Clone, PartialEq, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x38]
    pub struct PlayerPositionLookS2c {
        pub position: [f64; 3],
        pub yaw: f32,
        pub pitch: f32,
        pub flags: SyncPlayerPosLookFlags,
        pub teleport_id: VarInt,
        pub dismount_vehicle: bool,
    }

    #[derive(Clone, PartialEq, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x3a]
    pub struct EntitiesDestroyS2c<'a> {
        pub entity_ids: Cow<'a, [VarInt]>,
    }

    #[derive(Clone, PartialEq, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x3b]
    pub struct RemoveEntityStatusEffectS2c {
        pub entity_id: VarInt,
        pub effect_id: VarInt,
    }

    #[derive(Clone, PartialEq, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x3c]
    pub struct ResourcePackSendS2c<'a> {
        pub url: &'a str,
        pub hash: &'a str,
        pub forced: bool,
        pub prompt_message: Option<Cow<'a, Text>>,
    }

    #[derive(Clone, PartialEq, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x3d]
    pub struct PlayerRespawnS2c<'a> {
        pub dimension_type_name: Ident<&'a str>,
        pub dimension_name: Ident<&'a str>,
        pub hashed_seed: u64,
        pub game_mode: GameMode,
        pub previous_game_mode: i8,
        pub is_debug: bool,
        pub is_flat: bool,
        pub copy_metadata: bool,
        pub last_death_location: Option<GlobalPos<'a>>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x3e]
    pub struct EntitySetHeadYawS2c {
        pub entity_id: VarInt,
        pub head_yaw: ByteAngle,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x3f]
    pub struct ChunkDeltaUpdateS2c<'a> {
        pub chunk_section_position: i64,
        pub invert_trust_edges: bool,
        pub blocks: Cow<'a, [VarLong]>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x40]
    pub struct SelectAdvancementsTabS2c<'a> {
        pub identifier: Option<Ident<&'a str>>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x41]
    pub struct ServerMetadataS2c<'a> {
        pub motd: Option<Cow<'a, Text>>,
        pub icon: Option<&'a str>,
        pub enforce_secure_chat: bool,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x42]
    pub struct OverlayMessageS2c<'a> {
        pub action_bar_text: Cow<'a, Text>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x43]
    pub struct WorldBorderCenterChangedS2c {
        pub xz_position: [f64; 2],
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x44]
    pub struct WorldBorderInterpolateSizeS2c {
        pub old_diameter: f64,
        pub new_diameter: f64,
        pub speed: VarLong,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x45]
    pub struct WorldBorderSizeChangedS2c {
        pub diameter: f64,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x46]
    pub struct WorldBorderWarningTimeChangedS2c {
        pub warning_time: VarInt,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x47]
    pub struct WorldBorderWarningBlocksChangedS2c {
        pub warning_blocks: VarInt,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x48]
    pub struct SetCameraEntityS2c {
        pub entity_id: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x49]
    pub struct UpdateSelectedSlotS2c {
        pub slot: u8,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x4a]
    pub struct ChunkRenderDistanceCenterS2c {
        pub chunk_x: VarInt,
        pub chunk_z: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x4b]
    pub struct ChunkLoadDistanceS2c {
        pub view_distance: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x4c]
    pub struct PlayerSpawnPositionS2c {
        pub position: BlockPos,
        pub angle: f32,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x4d]
    pub struct ScoreboardDisplayS2c<'a> {
        pub position: u8,
        pub score_name: &'a str,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x4e]
    pub struct EntityTrackerUpdateS2c<'a> {
        pub entity_id: VarInt,
        pub metadata: RawBytes<'a>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x4f]
    pub struct EntityAttachS2c {
        pub attached_entity_id: i32,
        pub holding_entity_id: i32,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x50]
    pub struct EntityVelocityUpdateS2c {
        pub entity_id: VarInt,
        pub velocity: [i16; 3],
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x52]
    pub struct ExperienceBarUpdateS2c {
        pub bar: f32,
        pub level: VarInt,
        pub total_xp: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x53]
    pub struct HealthUpdateS2c {
        pub health: f32,
        pub food: VarInt,
        pub food_saturation: f32,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x54]
    pub struct ScoreboardObjectiveUpdateS2c<'a> {
        pub objective_name: &'a str,
        pub mode: UpdateObjectiveMode,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x55]
    pub struct EntityPassengersSetS2c {
        /// Vehicle's entity id
        pub entity_id: VarInt,
        pub passengers: Vec<VarInt>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x56]
    pub struct TeamS2c<'a> {
        pub team_name: &'a str,
        pub mode: UpdateTeamsMode<'a>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x57]
    pub struct ScoreboardPlayerUpdateS2c<'a> {
        pub entity_name: &'a str,
        pub action: UpdateScoreAction<'a>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x58]
    pub struct SimulationDistanceS2c {
        pub simulation_distance: VarInt,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x59]
    pub struct SubtitleS2c<'a> {
        pub subtitle_text: Cow<'a, Text>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x5a]
    pub struct WorldTimeUpdateS2c {
        /// The age of the world in 1/20ths of a second.
        pub world_age: i64,
        /// The current time of day in 1/20ths of a second.
        /// The value should be in the range \[0, 24000].
        /// 6000 is noon, 12000 is sunset, and 18000 is midnight.
        pub time_of_day: i64,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x5b]
    pub struct TitleS2c<'a> {
        pub title_text: Cow<'a, Text>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x5c]
    pub struct TitleFadeS2c {
        /// Ticks to spend fading in.
        pub fade_in: i32,
        /// Ticks to keep the title displayed.
        pub stay: i32,
        /// Ticks to spend fading out.
        pub fade_out: i32,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x5d]
    pub struct PlaySoundFromEntityS2c {
        pub id: VarInt,
        pub category: SoundCategory,
        pub entity_id: VarInt,
        pub volume: f32,
        pub pitch: f32,
        pub seed: i64,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x5e]
    pub struct PlaySoundS2c<'a> {
        pub id: SoundId<'a>,
        pub category: SoundCategory,
        pub position: [i32; 3],
        pub volume: f32,
        pub pitch: f32,
        pub seed: i64,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x60]
    pub struct GameMessageS2c<'a> {
        pub chat: Cow<'a, Text>,
        /// Whether the message is in the actionbar or the chat.
        pub overlay: bool,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x61]
    pub struct PlayerListHeaderS2c<'a> {
        pub header: Cow<'a, Text>,
        pub footer: Cow<'a, Text>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x62]
    pub struct NbtQueryResponseS2c {
        pub transaction_id: VarInt,
        pub nbt: Compound,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x63]
    pub struct ItemPickupAnimationS2c {
        pub collected_entity_id: VarInt,
        pub collector_entity_id: VarInt,
        pub pickup_item_count: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x64]
    pub struct EntityPositionS2c {
        pub entity_id: VarInt,
        pub position: [f64; 3],
        pub yaw: ByteAngle,
        pub pitch: ByteAngle,
        pub on_ground: bool,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x66]
    pub struct EntityAttributesS2c<'a> {
        pub entity_id: VarInt,
        pub properties: Vec<AttributeProperty<'a>>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x67]
    pub struct FeaturesS2c<'a> {
        pub features: Vec<Ident<&'a str>>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x68]
    pub struct EntityStatusEffectS2c {
        pub entity_id: VarInt,
        pub effect_id: VarInt,
        pub amplifier: u8,
        pub duration: VarInt,
        pub flags: EntityEffectFlags,
        pub factor_codec: Option<Compound>,
    }

    #[derive(Clone, Debug, Encode, Decode, EncodePacket, DecodePacket)]
    #[packet_id = 0x69]
    pub struct SynchronizeRecipesS2c<'a> {
        pub recipes: Vec<DeclaredRecipe<'a>>,
    }

    #[derive(Clone, Debug, Encode, Decode, EncodePacket, DecodePacket)]
    #[packet_id = 0x6a]
    pub struct SynchronizeTagsS2c<'a> {
        pub tags: Vec<TagGroup<'a>>,
    }

    packet_enum! {
        #[derive(Clone)]
        S2cPlayPacket<'a> {
            EntitySpawnS2c,
            ExperienceOrbSpawnS2c,
            PlayerSpawnS2c,
            EntityAnimationS2c,
            StatisticsS2c,
            PlayerActionResponseS2c,
            BlockBreakingProgressS2c,
            BlockEntityUpdateS2c<'a>,
            BlockEventS2c,
            BlockUpdateS2c,
            BossBarS2c,
            DifficultyS2c,
            ClearTitlesS2c,
            CommandSuggestionsS2c<'a>,
            CommandTreeS2c<'a>,
            CloseScreenS2c,
            InventoryS2c<'a>,
            ScreenHandlerPropertyUpdateS2c,
            ScreenHandlerSlotUpdateS2c<'a>,
            CooldownUpdateS2c,
            ChatSuggestionsS2c<'a>,
            CustomPayloadS2c<'a>,
            RemoveMessageS2c<'a>,
            DisconnectS2c<'a>,
            ProfilelessChatMessageS2c<'a>,
            EntityStatusS2c,
            ExplosionS2c<'a>,
            UnloadChunkS2c,
            GameStateChangeS2c,
            OpenHorseScreenS2c,
            WorldBorderInitializeS2c,
            KeepAliveS2c,
            ChunkDataS2c<'a>,
            WorldEventS2c,
            LightUpdateS2c,
            ParticleS2c,
            GameJoinS2c<'a>,
            MapUpdateS2c<'a>,
            SetTradeOffersS2c,
            MoveRelativeS2c,
            RotateAndMoveRelativeS2c,
            RotateS2c,
            VehicleMoveS2c,
            OpenWrittenBookS2c,
            OpenScreenS2c<'a>,
            SignEditorOpen,
            PlayPingS2c,
            CraftFailedResponseS2c<'a>,
            PlayerAbilitiesS2c,
            ChatMessageS2c<'a>,
            EndCombatS2c,
            EnterCombatS2c,
            DeathMessageS2c<'a>,
            PlayerRemove<'a>,
            PlayerListS2c<'a>,
            LookAtS2c,
            PlayerPositionLookS2c,
            UnlockRecipesS2c<'a>,
            EntitiesDestroyS2c<'a>,
            RemoveEntityStatusEffectS2c,
            ResourcePackSendS2c<'a>,
            PlayerRespawnS2c<'a>,
            EntitySetHeadYawS2c,
            ChunkDeltaUpdateS2c<'a>,
            SelectAdvancementsTabS2c<'a>,
            ServerMetadataS2c<'a>,
            OverlayMessageS2c<'a>,
            WorldBorderCenterChangedS2c,
            WorldBorderInterpolateSizeS2c,
            WorldBorderSizeChangedS2c,
            WorldBorderWarningTimeChangedS2c,
            WorldBorderWarningBlocksChangedS2c,
            SetCameraEntityS2c,
            UpdateSelectedSlotS2c,
            ChunkRenderDistanceCenterS2c,
            ChunkLoadDistanceS2c,
            PlayerSpawnPositionS2c,
            ScoreboardDisplayS2c<'a>,
            EntityTrackerUpdateS2c<'a>,
            EntityAttachS2c,
            EntityVelocityUpdateS2c,
            EntityEquipmentUpdateS2c,
            ExperienceBarUpdateS2c,
            HealthUpdateS2c,
            ScoreboardObjectiveUpdateS2c<'a>,
            EntityPassengersSetS2c,
            TeamS2c<'a>,
            ScoreboardPlayerUpdateS2c<'a>,
            SimulationDistanceS2c,
            SubtitleS2c<'a>,
            WorldTimeUpdateS2c,
            TitleS2c<'a>,
            TitleFadeS2c,
            PlaySoundFromEntityS2c,
            PlaySoundS2c<'a>,
            StopSoundS2c<'a>,
            GameMessageS2c<'a>,
            PlayerListHeaderS2c<'a>,
            NbtQueryResponseS2c,
            ItemPickupAnimationS2c,
            EntityPositionS2c,
            AdvancementUpdateS2c<'a>,
            EntityAttributesS2c<'a>,
            FeaturesS2c<'a>,
            EntityStatusEffectS2c,
            SynchronizeRecipesS2c<'a>,
            SynchronizeTagsS2c<'a>,
        }
    }
}
