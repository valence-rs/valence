//! Server to client packets.

use super::*;
use crate::particle::ParticleS2c;

pub mod status {
    use super::*;

    def_struct! {
        StatusResponse {
            json_response: String
        }
    }

    def_struct! {
        PingResponse {
            /// Should be the same as the payload from ping.
            payload: u64
        }
    }

    def_packet_group! {
        S2cStatusPacket {
            StatusResponse = 0,
            PingResponse = 1,
        }
    }
}

pub mod login {
    use super::*;

    def_struct! {
        DisconnectLogin {
            reason: Text,
        }
    }

    def_struct! {
        EncryptionRequest {
            /// Currently unused
            server_id: BoundedString<0, 20>,
            /// The RSA public key
            public_key: Vec<u8>,
            verify_token: BoundedArray<u8, 4, 16>,
        }
    }

    def_struct! {
        LoginSuccess {
            uuid: Uuid,
            username: Username<String>,
            properties: Vec<Property>,
        }
    }

    def_struct! {
        SetCompression {
            threshold: VarInt
        }
    }

    def_struct! {
        LoginPluginRequest {
            message_id: VarInt,
            channel: Ident<String>,
            data: RawBytes,
        }
    }

    def_packet_group! {
        S2cLoginPacket {
            DisconnectLogin = 0,
            EncryptionRequest = 1,
            LoginSuccess = 2,
            SetCompression = 3,
            LoginPluginRequest = 4,
        }
    }
}

pub mod play {
    use super::*;

    def_struct! {
        SpawnEntity {
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
        SpawnExperienceOrb {
            entity_id: VarInt,
            position: Vec3<f64>,
            count: i16,
        }
    }

    def_struct! {
        SpawnPlayer {
            entity_id: VarInt,
            player_uuid: Uuid,
            position: Vec3<f64>,
            yaw: ByteAngle,
            pitch: ByteAngle,
        }
    }

    def_struct! {
        EntityAnimationS2c {
            entity_id: VarInt,
            animation: u8,
        }
    }

    def_struct! {
        AcknowledgeBlockChange {
            sequence: VarInt,
        }
    }

    def_struct! {
        SetBlockDestroyStage {
            entity_id: VarInt,
            location: BlockPos,
            destroy_stage: BoundedInt<u8, 0, 10>,
        }
    }

    def_struct! {
        BlockEntityData {
            location: BlockPos,
            kind: VarInt, // TODO: use enum here
            data: Compound,
        }
    }

    def_struct! {
        BlockAction {
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

    def_struct! {
        SetContainerContent {
            window_id: u8,
            state_id: VarInt,
            slots: Vec<Slot>,
            carried_item: Slot,
        }
    }

    def_struct! {
        SetContainerProperty {
            window_id: u8,
            property: i16,
            value: i16,
        }
    }

    def_struct! {
        SetContainerSlot {
            window_id: i8,
            state_id: VarInt,
            slot_idx: i16,
            slot_data: Slot,
        }
    }

    def_struct! {
        SetCooldown {
            item_id: VarInt,
            cooldown_ticks: VarInt,
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
        PluginMessageS2c {
            channel: Ident<String>,
            data: RawBytes,
        }
    }

    def_struct! {
        CustomSoundEffect {
            name: Ident<String>,
            category: SoundCategory,
            position: Vec3<i32>,
            volume: f32,
            pitch: f32,
            seed: i64,
        }
    }

    def_struct! {
        DisconnectPlay {
            reason: Text,
        }
    }

    def_struct! {
        EntityEvent {
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
        GameEvent {
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
        KeepAliveS2c {
            id: i64,
        }
    }

    def_struct! {
        ChunkDataAndUpdateLight {
            chunk_x: i32,
            chunk_z: i32,
            heightmaps: Compound,
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

    def_struct! {
        ChunkDataBlockEntity {
            packed_xz: i8,
            y: i16,
            kind: VarInt,
            data: Compound,
        }
    }

    def_struct! {
        LoginPlay {
            /// Entity ID of the joining player
            entity_id: i32,
            is_hardcore: bool,
            gamemode: GameMode,
            previous_gamemode: GameMode,
            dimension_names: Vec<Ident<String>>,
            /// Contains information about dimensions, biomes, and chats.
            registry_codec: Compound,
            /// The name of the dimension type being spawned into.
            dimension_type_name: Ident<String>,
            /// The name of the dimension being spawned into.
            dimension_name: Ident<String>,
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
            last_death_location: Option<(Ident<String>, BlockPos)>,
        }
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
        UpdateEntityPosition {
            entity_id: VarInt,
            delta: Vec3<i16>,
            on_ground: bool,
        }
    }

    def_struct! {
        UpdateEntityPositionAndRotation {
            entity_id: VarInt,
            delta: Vec3<i16>,
            yaw: ByteAngle,
            pitch: ByteAngle,
            on_ground: bool,
        }
    }

    def_struct! {
        UpdateEntityRotation {
            entity_id: VarInt,
            yaw: ByteAngle,
            pitch: ByteAngle,
            on_ground: bool,
        }
    }

    def_struct! {
        OpenScreen {
            window_id: VarInt,
            window_type: VarInt,
            window_title: Text,
        }
    }

    def_struct! {
        PlayerChatMessage {
            // TODO: more 1.19 stuff.
            message: Text,
            /// Index into the chat type registry
            kind: VarInt,
            sender: Uuid,
        }
    }

    def_struct! {
        CombatDeath {
            player_id: VarInt,
            /// Killer's entity ID, -1 if no killer
            entity_id: i32,
            message: Text
        }
    }

    def_enum! {
        PlayerInfo: VarInt {
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
        SynchronizePlayerPosition {
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
        RemoveEntities {
            entities: Vec<VarInt>,
        }
    }

    def_struct! {
        ResourcePackS2c {
            url: String,
            hash: BoundedString<0, 40>,
            forced: bool,
            prompt_message: Option<Text>,
        }
    }

    def_struct! {
        Respawn {
            dimension_type_name: Ident<String>,
            dimension_name: Ident<String>,
            hashed_seed: u64,
            game_mode: GameMode,
            previous_game_mode: GameMode,
            is_debug: bool,
            is_flat: bool,
            copy_metadata: bool,
            last_death_location: Option<(Ident<String>, BlockPos)>,
        }
    }

    def_struct! {
        SetHeadRotation {
            entity_id: VarInt,
            head_yaw: ByteAngle,
        }
    }

    def_struct! {
        UpdateSectionBlocks {
            chunk_section_position: i64,
            invert_trust_edges: bool,
            blocks: Vec<VarLong>,
        }
    }

    def_struct! {
        SetActionBarText {
            text: Text
        }
    }

    def_struct! {
        SetHeldItemS2c {
            slot: BoundedInt<u8, 0, 9>,
        }
    }

    def_struct! {
        SetCenterChunk {
            chunk_x: VarInt,
            chunk_z: VarInt,
        }
    }

    def_struct! {
        SetRenderDistance {
            view_distance: BoundedInt<VarInt, 2, 32>,
        }
    }

    def_struct! {
        SetDefaultSpawnPosition {
            location: BlockPos,
            angle: f32,
        }
    }

    def_struct! {
        SetEntityMetadata {
            entity_id: VarInt,
            metadata: RawBytes,
        }
    }

    def_struct! {
        SetEntityVelocity {
            entity_id: VarInt,
            velocity: Vec3<i16>,
        }
    }

    def_struct! {
        SetExperience {
            bar: f32,
            level: VarInt,
            total_xp: VarInt,
        }
    }

    def_struct! {
        SetHealth {
            health: f32,
            food: VarInt,
            food_saturation: f32,
        }
    }

    def_struct! {
        SetSubtitleText {
            subtitle_text: Text,
        }
    }

    def_struct! {
        UpdateTime {
            /// The age of the world in 1/20ths of a second.
            world_age: i64,
            /// The current time of day in 1/20ths of a second.
            /// The value should be in the range \[0, 24000].
            /// 6000 is noon, 12000 is sunset, and 18000 is midnight.
            time_of_day: i64,
        }
    }

    def_struct! {
        SetTitleText {
            text: Text,
        }
    }

    def_struct! {
        #[derive(Copy, PartialEq, Eq)]
        SetTitleAnimationTimes {
            /// Ticks to spend fading in.
            fade_in: u32,
            /// Ticks to keep the title displayed.
            stay: u32,
            /// Ticks to spend fading out.
            fade_out: u32,
        }
    }

    def_struct! {
        EntitySoundEffect {
            id: VarInt,
            category: SoundCategory,
            entity_id: VarInt,
            volume: f32,
            pitch: f32
        }
    }

    def_struct! {
        SoundEffect {
            id: VarInt,
            category: SoundCategory,
            position: Vec3<i32>,
            volume: f32,
            pitch: f32,
            seed: i64
        }
    }

    def_struct! {
        SystemChatMessage {
            chat: Text,
            /// Index into the chat type registry.
            kind: VarInt,
        }
    }

    def_struct! {
        SetTabListHeaderAndFooter {
            header: Text,
            footer: Text,
        }
    }

    def_struct! {
        TeleportEntity {
            entity_id: VarInt,
            position: Vec3<f64>,
            yaw: ByteAngle,
            pitch: ByteAngle,
            on_ground: bool,
        }
    }

    def_struct! {
        UpdateAttributes {
            entity_id: VarInt,
            properties: Vec<EntityAttributesProperty>,
        }
    }

    def_struct! {
        EntityAttributesProperty {
            key: Ident<String>,
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
            SpawnEntity = 0,
            SpawnExperienceOrb = 1,
            SpawnPlayer = 2,
            EntityAnimationS2c = 3,
            AcknowledgeBlockChange = 5,
            SetBlockDestroyStage = 6,
            BlockEntityData = 7,
            BlockAction = 8,
            BlockUpdate = 9,
            BossBar = 10,
            ClearTitles = 13,
            PluginMessageS2c = 22,
            SetContainerContent = 17,
            SetContainerProperty = 18,
            SetContainerSlot = 19,
            SetCooldown = 20,
            CustomSoundEffect = 23,
            DisconnectPlay = 25,
            EntityEvent = 26,
            UnloadChunk = 28,
            GameEvent = 29,
            KeepAliveS2c = 32,
            ChunkDataAndUpdateLight = 33,
            ParticleS2c = 35,
            LoginPlay = 37,
            UpdateEntityPosition = 40,
            UpdateEntityPositionAndRotation = 41,
            UpdateEntityRotation = 42,
            OpenScreen = 45,
            PlayerChatMessage = 51,
            CombatDeath = 54,
            PlayerInfo = 55,
            SynchronizePlayerPosition = 57,
            RemoveEntities = 59,
            ResourcePackS2c = 61,
            Respawn = 62,
            SetHeadRotation = 63,
            UpdateSectionBlocks = 64,
            SetActionBarText = 67,
            SetHeldItemS2c = 74,
            SetCenterChunk = 75,
            SetRenderDistance = 76,
            SetDefaultSpawnPosition = 77,
            SetEntityMetadata = 80,
            SetEntityVelocity = 82,
            SetExperience = 84,
            SetHealth = 85,
            SetSubtitleText = 91,
            UpdateTime = 92,
            SetTitleText = 93,
            SetTitleAnimationTimes = 94,
            EntitySoundEffect = 95,
            SoundEffect = 96,
            SystemChatMessage = 98,
            SetTabListHeaderAndFooter = 99,
            TeleportEntity = 102,
            UpdateAttributes = 104,
        }
    }
}
