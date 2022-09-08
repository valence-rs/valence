//! Server to client packets.

use super::*;

pub mod status {
    use super::*;

    def_struct! {
        QueryResponse {
            json_response: String
        }
    }

    def_struct! {
        QueryPong {
            /// Should be the same as the payload from ping.
            payload: u64
        }
    }

    def_packet_group! {
        S2cStatusPacket {
            QueryResponse = 0,
            QueryPong = 1,
        }
    }
}

pub mod login {
    use super::*;

    def_struct! {
        LoginDisconnect {
            reason: Text,
        }
    }

    def_struct! {
        EncryptionRequest {
            /// Currently unused
            server_id: BoundedString<0, 20>,
            /// The RSA public key
            public_key: Vec<u8>,
            verify_token: BoundedArray<u8, 16, 16>,
        }
    }

    def_struct! {
        LoginSuccess {
            uuid: Uuid,
            username: BoundedString<3, 16>,
            properties: Vec<Property>,
        }
    }

    def_struct! {
        LoginCompression {
            threshold: VarInt
        }
    }

    def_struct! {
        LoginPluginRequest {
            message_id: VarInt,
            channel: Ident,
            data: RawBytes,
        }
    }

    def_packet_group! {
        S2cLoginPacket {
            LoginDisconnect = 0,
            EncryptionRequest = 1,
            LoginSuccess = 2,
            LoginCompression = 3,
            LoginPluginRequest = 4,
        }
    }
}

pub mod play {
    use super::*;

    def_struct! {
        EntitySpawn {
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
        ExperienceOrbSpawn {
            entity_id: VarInt,
            position: Vec3<f64>,
            count: i16,
        }
    }

    def_struct! {
        PlayerSpawn {
            entity_id: VarInt,
            player_uuid: Uuid,
            position: Vec3<f64>,
            yaw: ByteAngle,
            pitch: ByteAngle,
        }
    }

    def_struct! {
        EntityAnimation {
            entity_id: VarInt,
            animation: u8,
        }
    }

    def_struct! {
        PlayerActionResponse {
            sequence: VarInt,
        }
    }

    def_struct! {
        BlockBreakingProgress {
            entity_id: VarInt,
            location: BlockPos,
            destroy_stage: BoundedInt<u8, 0, 10>,
        }
    }

    def_struct! {
        BlockEntityUpdate {
            location: BlockPos,
            kind: VarInt, // TODO: use enum here
            data: Compound,
        }
    }

    def_struct! {
        BlockEvent {
            location: BlockPos,
            action_id: u8,
            action_param: u8,
            block_type: VarInt, // TODO: use BlockType type.
        }
    }

    def_struct! {
        BlockUpdate {
            location: BlockPos,
            block_id: VarInt,
        }
    }

    def_struct! {
        BossBar {
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
        SetDifficulty {
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
        ClearTitles {
            reset: bool,
        }
    }

    def_enum! {
        SoundCategory: VarInt {
            Master = 0,
            Music = 1,
            Record = 2,
            Weather = 3,
            Block = 4,
            Hostile = 5,
            Neutral = 6,
            Player = 7,
            Ambient = 8,
            Voice = 9,
        }
    }

    def_struct! {
        PlaySoundId {
            name: Ident,
            category: SoundCategory,
            position: Vec3<i32>,
            volume: f32,
            pitch: f32,
            seed: i64,
        }
    }

    def_struct! {
        Disconnect {
            reason: Text,
        }
    }

    def_struct! {
        EntityStatus {
            entity_id: i32,
            entity_status: u8,
        }
    }

    def_struct! {
        UnloadChunk {
            chunk_x: i32,
            chunk_z: i32
        }
    }

    def_struct! {
        GameStateChange {
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
        WorldBorderInitialize {
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
        KeepAlive {
            id: i64,
        }
    }

    def_struct! {
        ChunkData {
            chunk_x: i32,
            chunk_z: i32,
            heightmaps: NbtBridge<ChunkDataHeightmaps>,
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
        #[serde(rename = "MOTION_BLOCKING", with = "crate::nbt::long_array")]
        pub motion_blocking: Vec<i64>,
    }

    def_struct! {
        ChunkDataBlockEntity {
            packed_xz: i8,
            y: i16,
            kind: VarInt,
            data: Compound,
        }
    }

    def_struct! {
        GameJoin {
            /// Entity ID of the joining player
            entity_id: i32,
            is_hardcore: bool,
            gamemode: GameMode,
            previous_gamemode: GameMode,
            dimension_names: Vec<Ident>,
            registry_codec: NbtBridge<RegistryCodec>,
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
        MoveRelative {
            entity_id: VarInt,
            delta: Vec3<i16>,
            on_ground: bool,
        }
    }

    def_struct! {
        RotateAndMoveRelative {
            entity_id: VarInt,
            delta: Vec3<i16>,
            yaw: ByteAngle,
            pitch: ByteAngle,
            on_ground: bool,
        }
    }

    def_struct! {
        Rotate {
            entity_id: VarInt,
            yaw: ByteAngle,
            pitch: ByteAngle,
            on_ground: bool,
        }
    }

    def_struct! {
        ChatMessage {
            // TODO: more 1.19 stuff.
            message: Text,
            /// Index into the chat type registry
            kind: VarInt,
            sender: Uuid,
        }
    }

    def_enum! {
        UpdatePlayerList: VarInt {
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
            sig_data: Option<PublicKeyData>,
        }
    }

    def_struct! {
        PlayerPositionLook {
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
        EntitiesDestroy {
            entities: Vec<VarInt>,
        }
    }

    def_struct! {
        PlayerRespawn {
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
        EntitySetHeadYaw {
            entity_id: VarInt,
            head_yaw: ByteAngle,
        }
    }

    def_struct! {
        ChunkSectionUpdate {
            chunk_section_position: i64,
            invert_trust_edges: bool,
            blocks: Vec<VarLong>,
        }
    }

    def_struct! {
        OverlayMessage {
            text: Text
        }
    }

    def_struct! {
        UpdateSelectedSlot {
            slot: BoundedInt<u8, 0, 9>,
        }
    }

    def_struct! {
        ChunkRenderDistanceCenter {
            chunk_x: VarInt,
            chunk_z: VarInt,
        }
    }

    def_struct! {
        ChunkLoadDistance {
            view_distance: BoundedInt<VarInt, 2, 32>,
        }
    }

    def_struct! {
        PlayerSpawnPosition {
            location: BlockPos,
            angle: f32,
        }
    }

    def_struct! {
        EntityTrackerUpdate {
            entity_id: VarInt,
            metadata: RawBytes,
        }
    }

    def_struct! {
        EntityVelocityUpdate {
            entity_id: VarInt,
            velocity: Vec3<i16>,
        }
    }

    def_struct! {
        UpdateSubtitle {
            subtitle_text: Text,
        }
    }

    def_struct! {
        WorldTimeUpdate {
            /// The age of the world in 1/20ths of a second.
            world_age: i64,
            /// The current time of day in 1/20ths of a second.
            /// The value should be in the range \[0, 24000].
            /// 6000 is noon, 12000 is sunset, and 18000 is midnight.
            time_of_day: i64,
        }
    }

    def_struct! {
        UpdateTitle {
            text: Text,
        }
    }

    def_struct! {
        #[derive(Copy, PartialEq, Eq)]
        TitleFade {
            /// Ticks to spend fading in.
            fade_in: u32,
            /// Ticks to keep the title displayed.
            stay: u32,
            /// Ticks to spend fading out.
            fade_out: u32,
        }
    }

    def_struct! {
        PlaySoundFromEntity {
            id: VarInt,
            category: SoundCategory,
            entity_id: VarInt,
            volume: f32,
            pitch: f32
        }
    }

    def_struct! {
        PlaySound {
            id: VarInt,
            category: SoundCategory,
            position: Vec3<i32>,
            volume: f32,
            pitch: f32,
            seed: i64
        }
    }

    def_struct! {
        GameMessage {
            chat: Text,
            /// Index into the chat type registry.
            kind: VarInt,
        }
    }

    def_struct! {
        PlayerListHeaderFooter {
            header: Text,
            footer: Text,
        }
    }

    def_struct! {
        EntityPosition {
            entity_id: VarInt,
            position: Vec3<f64>,
            yaw: ByteAngle,
            pitch: ByteAngle,
            on_ground: bool,
        }
    }

    def_struct! {
        EntityAttributes {
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
            EntitySpawn = 0,
            ExperienceOrbSpawn = 1,
            PlayerSpawn = 2,
            EntityAnimation = 3,
            PlayerActionResponse = 5,
            BlockBreakingProgress = 6,
            BlockEntityUpdate = 7,
            BlockEvent = 8,
            BlockUpdate = 9,
            BossBar = 10,
            ClearTitles = 13,
            PlaySoundId = 23,
            Disconnect = 25,
            EntityStatus = 26,
            UnloadChunk = 28,
            GameStateChange = 29,
            KeepAlive = 32,
            ChunkData = 33,
            GameJoin = 37,
            MoveRelative = 40,
            RotateAndMoveRelative = 41,
            Rotate = 42,
            ChatMessage = 51,
            UpdatePlayerList = 55,
            PlayerPositionLook = 57,
            EntitiesDestroy = 59,
            PlayerRespawn = 62,
            EntitySetHeadYaw = 63,
            ChunkSectionUpdate = 64,
            OverlayMessage = 67,
            UpdateSelectedSlot = 74,
            ChunkRenderDistanceCenter = 75,
            ChunkLoadDistance = 76,
            PlayerSpawnPosition = 77,
            EntityTrackerUpdate = 80,
            EntityVelocityUpdate = 82,
            UpdateSubtitle = 91,
            WorldTimeUpdate = 92,
            UpdateTitle = 93,
            TitleFade = 94,
            PlaySoundFromEntity = 95,
            PlaySound = 96,
            GameMessage = 98,
            PlayerListHeaderFooter = 99,
            EntityPosition = 102,
            EntityAttributes = 104,
        }
    }
}
