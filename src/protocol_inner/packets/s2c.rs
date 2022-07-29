//! Server to client packets.

use super::*;

pub mod status {
    use super::*;

    def_struct! {
        QueryResponse 0x00 {
            json_response: String
        }
    }

    def_struct! {
        QueryPong 0x01 {
            /// Should be the same as the payload from ping.
            payload: u64
        }
    }
}

pub mod login {
    use super::*;

    def_struct! {
        LoginDisconnect 0x00 {
            reason: Text,
        }
    }

    def_struct! {
        EncryptionRequest 0x01 {
            /// Currently unused
            server_id: BoundedString<0, 20>,
            /// The RSA public key
            public_key: Vec<u8>,
            verify_token: BoundedArray<u8, 16, 16>,
        }
    }

    def_struct! {
        LoginSuccess 0x02 {
            uuid: Uuid,
            username: BoundedString<3, 16>,
            properties: Vec<Property>,
        }
    }

    def_struct! {
        LoginCompression 0x03 {
            threshold: VarInt
        }
    }

    def_struct! {
        LoginPluginRequest 0x04 {
            message_id: VarInt,
            channel: Ident,
            data: RawBytes,
        }
    }

    def_packet_group! {
        S2cLoginPacket {
            LoginDisconnect,
            EncryptionRequest,
            LoginSuccess,
            LoginCompression,
            LoginPluginRequest,
        }
    }
}

pub mod play {
    use super::*;

    def_struct! {
        EntitySpawn 0x00 {
            entity_id: VarInt,
            object_uuid: Uuid,
            kind: VarInt,
            position: Vec3<f64>,
            pitch: ByteAngle,
            yaw: ByteAngle,
            head_yaw: ByteAngle,
            data: VarInt,
            velocity: Vec3<i16>,
        }
    }

    def_struct! {
        ExperienceOrbSpawn 0x01 {
            entity_id: VarInt,
            position: Vec3<f64>,
            count: i16,
        }
    }

    def_struct! {
        PlayerSpawn 0x02 {
            entity_id: VarInt,
            player_uuid: Uuid,
            position: Vec3<f64>,
            yaw: ByteAngle,
            pitch: ByteAngle,
        }
    }

    def_struct! {
        EntityAnimation 0x03 {
            entity_id: VarInt,
            animation: u8,
        }
    }

    def_struct! {
        PlayerActionResponse 0x05 {
            sequence: VarInt,
        }
    }

    def_struct! {
        BlockBreakingProgress 0x06 {
            entity_id: VarInt,
            location: BlockPos,
            destroy_stage: BoundedInt<u8, 0, 10>,
        }
    }

    def_struct! {
        BlockEntityUpdate 0x07 {
            location: BlockPos,
            kind: VarInt, // TODO: use enum here
            data: nbt::Blob,
        }
    }

    def_struct! {
        BlockEvent 0x08 {
            location: BlockPos,
            action_id: u8,
            action_param: u8,
            block_type: VarInt, // TODO: use BlockType type.
        }
    }

    def_struct! {
        BlockUpdate 0x09 {
            location: BlockPos,
            block_id: VarInt,
        }
    }

    def_struct! {
        BossBar 0x0a {
            uuid: Uuid,
            action: BossBarAction,
        }
    }

    def_enum! {
        BossBarAction: VarInt {
            Add: BossBarActionAdd = 0,
            // TODO
        }
    }

    def_struct! {
        BossBarActionAdd {
            title: Text,
            health: f32,
            color: BossBarColor,
            division: BossBarDivision,
            /// TODO: bitmask
            flags: u8,
        }
    }

    def_enum! {
        BossBarColor: VarInt {
            Pink = 0,
            Blue = 1,
            Red = 2,
            Green = 3,
            Yellow = 4,
            Purple = 5,
            White = 6,
        }
    }

    def_enum! {
        BossBarDivision: VarInt {
            NoDivision = 0,
            SixNotches = 1,
            TenNotches = 2,
            TwelveNotches = 3,
            TwentyNotches = 4,
        }
    }

    def_struct! {
        SetDifficulty 0x0b {
            difficulty: Difficulty,
            locked: bool,
        }
    }

    def_enum! {
        Difficulty: u8 {
            Peaceful = 0,
            Easy = 1,
            Normal = 2,
            Hard = 3,
        }
    }

    def_struct! {
        ClearTitles 0x0d {
            reset: bool,
        }
    }

    def_struct! {
        Disconnect 0x17 {
            reason: Text,
        }
    }

    def_struct! {
        EntityStatus 0x18 {
            entity_id: i32,
            entity_status: u8,
        }
    }

    def_struct! {
        UnloadChunk 0x1a {
            chunk_x: i32,
            chunk_z: i32
        }
    }

    def_struct! {
        GameStateChange 0x1b {
            reason: GameStateChangeReason,
            value: f32,
        }
    }

    def_enum! {
        GameStateChangeReason: u8 {
            NoRespawnBlockAvailable = 0,
            EndRaining = 1,
            BeginRaining = 2,
            ChangeGameMode = 3,
            WinGame = 4,
            DemoEvent = 5,
            ArrowHitPlayer = 6,
            RainLevelChange = 7,
            ThunderLevelChange = 8,
            PlayPufferfishStingSound = 9,
            PlayElderGuardianMobAppearance = 10,
            EnableRespawnScreen = 11,
        }
    }

    def_struct! {
        WorldBorderInitialize 0x1d {
            x: f64,
            z: f64,
            old_diameter: f64,
            new_diameter: f64,
            speed: VarLong,
            portal_teleport_boundary: VarInt,
            warning_blocks: VarInt,
            warning_time: VarInt,
        }
    }

    def_struct! {
        KeepAlive 0x1e {
            id: i64,
        }
    }

    def_struct! {
        ChunkData 0x1f {
            chunk_x: i32,
            chunk_z: i32,
            heightmaps: Nbt<ChunkDataHeightmaps>,
            blocks_and_biomes: Vec<u8>,
            block_entities: Vec<ChunkDataBlockEntity>,
            trust_edges: bool,
            sky_light_mask: BitVec<u64>,
            block_light_mask: BitVec<u64>,
            empty_sky_light_mask: BitVec<u64>,
            empty_block_light_mask: BitVec<u64>,
            sky_light_arrays: Vec<[u8; 2048]>,
            block_light_arrays: Vec<[u8; 2048]>,
        }
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ChunkDataHeightmaps {
        #[serde(rename = "MOTION_BLOCKING", serialize_with = "nbt::i64_array")]
        pub motion_blocking: Vec<i64>,
    }

    def_struct! {
        ChunkDataBlockEntity {
            packed_xz: i8,
            y: i16,
            kind: VarInt,
            data: nbt::Blob,
        }
    }

    def_struct! {
        GameJoin 0x23 {
            /// Entity ID of the joining player
            entity_id: i32,
            is_hardcore: bool,
            gamemode: GameMode,
            previous_gamemode: GameMode,
            dimension_names: Vec<Ident>,
            registry_codec: Nbt<RegistryCodec>,
            /// The name of the dimension type being spawned into.
            dimension_type_name: Ident,
            /// The name of the dimension being spawned into.
            dimension_name: Ident,
            /// Hash of the world's seed used for client biome noise.
            hashed_seed: i64,
            /// No longer used by the client.
            max_players: VarInt,
            view_distance: BoundedInt<VarInt, 2, 32>,
            simulation_distance: VarInt,
            /// If reduced debug info should be shown on the F3 screen.
            reduced_debug_info: bool,
            /// If player respawns should be instant or not.
            enable_respawn_screen: bool,
            is_debug: bool,
            /// If this is a superflat world.
            /// Superflat worlds have different void fog and horizon levels.
            is_flat: bool,
            last_death_location: Option<(Ident, BlockPos)>,
        }
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct RegistryCodec {
        #[serde(rename = "minecraft:dimension_type")]
        pub dimension_type_registry: DimensionTypeRegistry,
        #[serde(rename = "minecraft:worldgen/biome")]
        pub biome_registry: BiomeRegistry,
        #[serde(rename = "minecraft:chat_type")]
        pub chat_type_registry: ChatTypeRegistry,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct DimensionTypeRegistry {
        #[serde(rename = "type")]
        pub kind: Ident,
        pub value: Vec<DimensionTypeRegistryEntry>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct DimensionTypeRegistryEntry {
        pub name: Ident,
        pub id: i32,
        pub element: DimensionType,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct DimensionType {
        pub piglin_safe: bool,
        pub has_raids: bool,
        pub monster_spawn_light_level: i32,
        pub monster_spawn_block_light_limit: i32,
        pub natural: bool,
        pub ambient_light: f32,
        pub fixed_time: Option<i64>,
        pub infiniburn: String, // TODO: tag type?
        pub respawn_anchor_works: bool,
        pub has_skylight: bool,
        pub bed_works: bool,
        pub effects: Ident,
        pub min_y: i32,
        pub height: i32,
        pub logical_height: i32,
        pub coordinate_scale: f64,
        pub ultrawarm: bool,
        pub has_ceiling: bool,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct BiomeRegistry {
        #[serde(rename = "type")]
        pub kind: Ident,
        pub value: Vec<Biome>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Biome {
        pub name: Ident,
        pub id: i32,
        pub element: BiomeProperty,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct BiomeProperty {
        pub precipitation: String,
        pub depth: f32,
        pub temperature: f32,
        pub scale: f32,
        pub downfall: f32,
        pub category: String,
        pub temperature_modifier: Option<String>,
        pub effects: BiomeEffects,
        pub particle: Option<BiomeParticle>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct BiomeEffects {
        pub sky_color: i32,
        pub water_fog_color: i32,
        pub fog_color: i32,
        pub water_color: i32,
        pub foliage_color: Option<i32>,
        pub grass_color: Option<i32>,
        pub grass_color_modifier: Option<String>,
        pub music: Option<BiomeMusic>,
        pub ambient_sound: Option<Ident>,
        pub additions_sound: Option<BiomeAdditionsSound>,
        pub mood_sound: Option<BiomeMoodSound>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct BiomeMusic {
        pub replace_current_music: bool,
        pub sound: Ident,
        pub max_delay: i32,
        pub min_delay: i32,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct BiomeAdditionsSound {
        pub sound: Ident,
        pub tick_chance: f64,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct BiomeMoodSound {
        pub sound: Ident,
        pub tick_delay: i32,
        pub offset: f64,
        pub block_search_extent: i32,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct BiomeParticle {
        pub probability: f32,
        pub options: BiomeParticleOptions,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct BiomeParticleOptions {
        #[serde(rename = "type")]
        pub kind: Ident,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ChatTypeRegistry {
        #[serde(rename = "type")]
        pub kind: Ident,
        pub value: Vec<ChatTypeRegistryEntry>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ChatTypeRegistryEntry {
        pub name: Ident,
        pub id: i32,
        pub element: ChatType,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ChatType {
        pub chat: ChatTypeChat,
        pub narration: ChatTypeNarration,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ChatTypeChat {}

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ChatTypeNarration {
        pub priority: String,
    }

    def_enum! {
        #[derive(Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
        GameMode: u8 {
            #[default]
            Survival = 0,
            Creative = 1,
            Adventure = 2,
            Spectator = 3,
        }
    }

    def_struct! {
        MoveRelative 0x26 {
            entity_id: VarInt,
            delta: Vec3<i16>,
            on_ground: bool,
        }
    }

    def_struct! {
        RotateAndMoveRelative 0x27 {
            entity_id: VarInt,
            delta: Vec3<i16>,
            yaw: ByteAngle,
            pitch: ByteAngle,
            on_ground: bool,
        }
    }

    def_struct! {
        Rotate 0x28 {
            entity_id: VarInt,
            yaw: ByteAngle,
            pitch: ByteAngle,
            on_ground: bool,
        }
    }

    def_struct! {
        ChatMessage 0x30 {
            message: Text,
            /// Index into the chat type registry
            kind: VarInt,
            sender: Uuid,
            // TODO more fields
        }
    }

    def_enum! {
        UpdatePlayerList 0x34: VarInt {
            AddPlayer: Vec<PlayerListAddPlayer> = 0,
            UpdateGameMode: Vec<(Uuid, GameMode)> = 1,
            UpdateLatency: Vec<(Uuid, VarInt)> = 2,
            UpdateDisplayName: Vec<(Uuid, Option<Text>)> = 3,
            RemovePlayer: Vec<Uuid> = 4,
        }
    }

    def_struct! {
        PlayerListAddPlayer {
            uuid: Uuid,
            username: BoundedString<3, 16>,
            properties: Vec<Property>,
            game_mode: GameMode,
            ping: VarInt,
            display_name: Option<Text>,
            sig_data: Option<SignatureData>,
        }
    }

    def_struct! {
        PlayerPositionLook 0x36 {
            position: Vec3<f64>,
            yaw: f32,
            pitch: f32,
            flags: PlayerPositionLookFlags,
            teleport_id: VarInt,
            dismount_vehicle: bool,
        }
    }

    def_bitfield! {
        PlayerPositionLookFlags: u8 {
            x = 0,
            y = 1,
            z = 2,
            y_rot = 3,
            x_rot = 4,
        }
    }

    def_struct! {
        EntitiesDestroy 0x38 {
            entities: Vec<VarInt>,
        }
    }

    def_struct! {
        PlayerRespawn 0x3b {
            dimension_type_name: Ident,
            dimension_name: Ident,
            hashed_seed: u64,
            game_mode: GameMode,
            previous_game_mode: GameMode,
            is_debug: bool,
            is_flat: bool,
            copy_metadata: bool,
            last_death_location: Option<(Ident, BlockPos)>,
        }
    }

    def_struct! {
        EntitySetHeadYaw 0x3c {
            entity_id: VarInt,
            head_yaw: ByteAngle,
        }
    }

    def_struct! {
        ChunkSectionUpdate 0x3d {
            chunk_section_position: i64,
            invert_trust_edges: bool,
            blocks: Vec<VarLong>,
        }
    }

    def_struct! {
        UpdateSelectedSlot 0x47 {
            slot: BoundedInt<u8, 0, 9>,
        }
    }

    def_struct! {
        ChunkRenderDistanceCenter 0x48 {
            chunk_x: VarInt,
            chunk_z: VarInt,
        }
    }

    def_struct! {
        ChunkLoadDistance 0x49 {
            view_distance: BoundedInt<VarInt, 2, 32>,
        }
    }

    def_struct! {
        PlayerSpawnPosition 0x4a {
            location: BlockPos,
            angle: f32,
        }
    }

    def_struct! {
        EntityTrackerUpdate 0x4d {
            entity_id: VarInt,
            metadata: RawBytes,
        }
    }

    def_struct! {
        EntityVelocityUpdate 0x4f {
            entity_id: VarInt,
            velocity: Vec3<i16>,
        }
    }

    def_struct! {
        UpdateSubtitle 0x58 {
            subtitle_text: Text,
        }
    }

    def_struct! {
        WorldTimeUpdate 0x59 {
            /// The age of the world in 1/20ths of a second.
            world_age: i64,
            /// The current time of day in 1/20ths of a second.
            /// The value should be in the range \[0, 24000].
            /// 6000 is noon, 12000 is sunset, and 18000 is midnight.
            time_of_day: i64,
        }
    }

    def_struct! {
        UpdateTitle 0x5a {
            text: Text,
        }
    }

    def_struct! {
        #[derive(Copy, PartialEq, Eq)]
        TitleAnimationTimes 0x5b {
            /// Ticks to spend fading in.
            fade_in: u32,
            /// Ticks to keep the title displayed.
            stay: u32,
            /// Ticks to spend fading out.
            fade_out: u32,
        }
    }

    def_struct! {
        GameMessage 0x5f {
            chat: Text,
            /// Index into the chat type registry.
            kind: VarInt,
        }
    }

    def_struct! {
        PlayerListHeaderFooter 0x60 {
            header: Text,
            footer: Text,
        }
    }

    def_struct! {
        EntityPosition 0x63 {
            entity_id: VarInt,
            position: Vec3<f64>,
            yaw: ByteAngle,
            pitch: ByteAngle,
            on_ground: bool,
        }
    }

    def_struct! {
        EntityAttributes 0x65 {
            entity_id: VarInt,
            properties: Vec<EntityAttributesProperty>,
        }
    }

    def_struct! {
        EntityAttributesProperty {
            key: Ident,
            value: f64,
            modifiers: Vec<EntityAttributesModifiers>
        }
    }

    def_struct! {
        EntityAttributesModifiers {
            uuid: Uuid,
            amount: f64,
            operation: u8,
        }
    }

    def_packet_group! {
        S2cPlayPacket {
            EntitySpawn,
            ExperienceOrbSpawn,
            PlayerSpawn,
            EntityAnimation,
            PlayerActionResponse,
            BlockBreakingProgress,
            BlockEntityUpdate,
            BlockEvent,
            BlockUpdate,
            BossBar,
            ClearTitles,
            Disconnect,
            EntityStatus,
            UnloadChunk,
            GameStateChange,
            KeepAlive,
            ChunkData,
            GameJoin,
            MoveRelative,
            RotateAndMoveRelative,
            Rotate,
            ChatMessage,
            UpdatePlayerList,
            PlayerPositionLook,
            EntitiesDestroy,
            PlayerRespawn,
            EntitySetHeadYaw,
            ChunkSectionUpdate,
            UpdateSelectedSlot,
            ChunkRenderDistanceCenter,
            ChunkLoadDistance,
            PlayerSpawnPosition,
            EntityTrackerUpdate,
            EntityVelocityUpdate,
            UpdateSubtitle,
            WorldTimeUpdate,
            UpdateTitle,
            TitleAnimationTimes,
            GameMessage,
            PlayerListHeaderFooter,
            EntityPosition,
            EntityAttributes,
        }
    }
}
