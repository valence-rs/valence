//! Client to server packets.

use super::*;

pub mod handshake {
    use super::*;

    def_struct! {
        Handshake {
            protocol_version: VarInt,
            // by the minecraft protocol this is specified as a BoundedString<0, 255> but due
            // issues with bungeecord ip forwarding this limit is removed here and checked when handling the handshake
            server_address: String,
            server_port: u16,
            next_state: HandshakeNextState,
        }
    }

    def_enum! {
        HandshakeNextState: VarInt {
            Status = 1,
            Login = 2,
        }
    }

    def_packet_group! {
        C2sHandshakePacket {
            Handshake = 0,
        }
    }

    def_packet_group! {
        C2sBungeecordHandshakePacket {
            BungeecordHandshake = 0,
        }
    }
}

pub mod status {
    use super::*;

    def_struct! {
        QueryRequest {}
    }

    def_struct! {
        QueryPing {
            payload: u64
        }
    }

    def_packet_group! {
        C2sStatusPacket {
            QueryRequest = 0,
            QueryPing = 1,
        }
    }
}

pub mod login {
    use super::*;

    def_struct! {
        LoginStart {
            username: BoundedString<3, 16>,
            sig_data: Option<PublicKeyData>,
            profile_id: Option<Uuid>,
        }
    }

    def_struct! {
        EncryptionResponse {
            shared_secret: BoundedArray<u8, 16, 128>,
            token_or_sig: VerifyTokenOrMsgSig,
        }
    }

    def_enum! {
        VerifyTokenOrMsgSig: u8 {
            VerifyToken: BoundedArray<u8, 16, 128> = 1,
            MsgSig: MessageSignature = 0,
        }
    }

    def_struct! {
        MessageSignature {
            salt: u64,
            sig: Vec<u8>, // TODO: bounds?
        }
    }

    def_struct! {
        LoginPluginResponse {
            message_id: VarInt,
            data: Option<RawBytes>,
        }
    }

    def_packet_group! {
        C2sLoginPacket {
            LoginStart = 0,
            EncryptionResponse = 1,
            LoginPluginResponse = 2,
        }
    }
}

pub mod play {
    use super::super::*;

    def_struct! {
        TeleportConfirm {
            teleport_id: VarInt
        }
    }

    def_struct! {
        QueryBlockNbt {
            transaction_id: VarInt,
            location: BlockPos,
        }
    }

    def_enum! {
        UpdateDifficulty: i8 {
            Peaceful = 0,
            Easy = 1,
            Normal = 2,
            Hard = 3,
        }
    }

    def_struct! {
        MessageAcknowledgmentList {
            entries: Vec<MessageAcknowledgmentEntry>,
        }
    }

    def_struct! {
        MessageAcknowledgment {
            last_seen: MessageAcknowledgmentList,
            last_received: Option<MessageAcknowledgmentEntry>,
        }
    }

    def_struct! {
        MessageAcknowledgmentEntry {
            profile_id: Uuid,
            signature: Vec<u8>,
        }
    }

    def_struct! {
        ArgumentSignatureEntry {
            name: BoundedString<0, 16>,
            signature: Vec<u8>,
        }
    }

    def_struct! {
        CommandExecution {
            command: BoundedString<0, 256>,
            timestamp: u64,
            salt: u64,
            arg_sig: Vec<ArgumentSignatureEntry>,
            signed_preview: bool,
            acknowledgement: MessageAcknowledgment,
        }
    }

    def_struct! {
        ChatMessage {
            message: BoundedString<0, 256>,
            timestamp: u64,
            salt: u64,
            signature: Vec<u8>,
            signed_preview: bool,
            acknowledgement: MessageAcknowledgment,
        }
    }

    def_struct! {
        RequestChatPreview {
            query: i32, // TODO: is this an i32 or a varint?
            message: BoundedString<0, 256>,
        }
    }

    def_enum! {
        ClientStatus: VarInt {
            /// Sent when ready to complete login and ready to respawn after death.
            PerformRespawn = 0,
            /// Sent when the statistics menu is opened.
            RequestStatus = 1,
        }
    }

    def_struct! {
        ClientSettings {
            /// e.g. en_US
            locale: BoundedString<0, 16>,
            /// Client-side render distance in chunks.
            view_distance: BoundedInt<u8, 2, 32>,
            chat_mode: ChatMode,
            chat_colors: bool,
            displayed_skin_parts: DisplayedSkinParts,
            main_hand: MainHand,
            /// Currently always false
            enable_text_filtering: bool,
            /// False if the client should not show up in the hover preview.
            allow_server_listings: bool,
        }
    }

    def_enum! {
        #[derive(Copy, PartialEq, Eq)]
        ChatMode: VarInt {
            Enabled = 0,
            CommandsOnly = 1,
            Hidden = 2,
        }
    }

    def_bitfield! {
        DisplayedSkinParts: u8 {
            cape = 0,
            jacket = 1,
            left_sleeve = 2,
            right_sleeve = 3,
            left_pants_leg = 4,
            right_pants_leg = 5,
            hat = 6,
        }
    }

    def_enum! {
        #[derive(Copy, PartialEq, Eq)]
        MainHand: VarInt {
            Left = 0,
            Right = 1,
        }
    }

    def_struct! {
        RequestCommandCompletion {
            transaction_id: VarInt,
            /// Text behind the cursor without the '/'.
            text: BoundedString<0, 32500>
        }
    }

    def_struct! {
        ButtonClick {
            window_id: i8,
            button_id: i8,
        }
    }

    def_struct! {
        ClickSlot {
            // TODO
        }
    }

    def_struct! {
        CloseHandledScreen {
            window_id: u8,
        }
    }

    def_struct! {
        PluginMessageToServer {
            channel: Ident,
            data: BoundedArray<u8, 0, 32767>,
        }
    }

    def_struct! {
        BookUpdate {
            slot: VarInt,
            entries: Vec<String>,
            title: Option<String>,
        }
    }

    def_struct! {
        QueryEntityNbt {
            transaction_id: VarInt,
            entity_id: VarInt,
        }
    }

    def_struct! {
        PlayerInteractEntity {
            entity_id: VarInt,
            kind: InteractKind,
            sneaking: bool,
        }
    }

    def_enum! {
        InteractKind: VarInt {
            Interact: Hand = 0,
            Attack = 1,
            InteractAt: (Vec3<f32>, Hand) = 2
        }
    }

    def_enum! {
        #[derive(Copy, PartialEq, Eq)]
        Hand: VarInt {
            Main = 0,
            Off = 1,
        }
    }

    def_struct! {
        JigsawGenerate {
            location: BlockPos,
            levels: VarInt,
            keep_jigsaws: bool,
        }
    }

    def_struct! {
        KeepAlive {
            id: i64,
        }
    }

    def_struct! {
        UpdateDifficultyLock {
            locked: bool
        }
    }

    def_struct! {
        MovePlayerPosition {
            position: Vec3<f64>,
            on_ground: bool,
        }
    }

    def_struct! {
        MovePlayerPositionAndRotation {
            // Absolute position
            position: Vec3<f64>,
            /// Absolute rotation on X axis in degrees.
            yaw: f32,
            /// Absolute rotation on Y axis in degrees.
            pitch: f32,
            on_ground: bool,
        }
    }

    def_struct! {
        MovePlayerRotation {
            /// Absolute rotation on X axis in degrees.
            yaw: f32,
            /// Absolute rotation on Y axis in degrees.
            pitch: f32,
            on_ground: bool,
        }
    }

    def_struct! {
        MovePlayerOnGround {
            on_ground: bool
        }
    }

    def_struct! {
        MoveVehicle {
            /// Absolute position
            position: Vec3<f64>,
            /// Degrees
            yaw: f32,
            /// Degrees
            pitch: f32,
        }
    }

    def_struct! {
        BoatPaddleState {
            left_paddle_turning: bool,
            right_paddle_turning: bool,
        }
    }

    def_struct! {
        PickFromInventory {
            slot_to_use: VarInt,
        }
    }

    def_struct! {
        CraftRequest {
            window_id: i8,
            recipe: Ident,
            make_all: bool,
        }
    }

    def_enum! {
        UpdatePlayerAbilities: i8 {
            NotFlying = 0,
            Flying = 0b10,
        }
    }

    def_struct! {
        PlayerAction {
            status: DiggingStatus,
            location: BlockPos,
            face: BlockFace,
            sequence: VarInt,
        }
    }

    def_enum! {
        DiggingStatus: VarInt {
            StartedDigging = 0,
            CancelledDigging = 1,
            FinishedDigging = 2,
            DropItemStack = 3,
            DropItem = 4,
            ShootArrowOrFinishEating = 5,
            SwapItemInHand = 6,
        }
    }

    def_enum! {
        #[derive(Copy, PartialEq, Eq)]
        BlockFace: i8 {
            /// -Y
            Bottom = 0,
            /// +Y
            Top = 1,
            /// -Z
            North = 2,
            /// +Z
            South = 3,
            /// -X
            West = 4,
            /// +X
            East = 5,
        }
    }

    def_struct! {
        PlayerCommand {
            entity_id: VarInt,
            action_id: PlayerCommandId,
            jump_boost: BoundedInt<VarInt, 0, 100>,
        }
    }

    def_enum! {
        PlayerCommandId: VarInt {
            StartSneaking = 0,
            StopSneaking = 1,
            LeaveBed = 2,
            StartSprinting = 3,
            StopSprinting = 4,
            StartJumpWithHorse = 5,
            StopJumpWithHorse = 6,
            OpenHorseInventory = 7,
            StartFlyingWithElytra = 8,
        }
    }

    def_struct! {
        PlayerInput {
            sideways: f32,
            forward: f32,
            flags: PlayerInputFlags,
        }
    }

    def_bitfield! {
        PlayerInputFlags: u8 {
            jump = 0,
            unmount = 1,
        }
    }

    def_struct! {
        PlayPong {
            id: i32,
        }
    }

    def_struct! {
        RecipeBookChangeSettings {
            book_id: RecipeBookId,
            book_open: bool,
            filter_active: bool,
        }
    }

    def_enum! {
        RecipeBookId: VarInt {
            Crafting = 0,
            Furnace = 1,
            BlastFurnace = 2,
            Smoker = 3,
        }
    }

    def_struct! {
        RecipeBookSeenRecipe {
            recipe_id: Ident,
        }
    }

    def_struct! {
        RenameItem {
            item_name: BoundedString<0, 50>,
        }
    }

    def_enum! {
        ResourcePackStatus: VarInt {
            SuccessfullyLoaded = 0,
            Declined = 1,
            FailedDownload = 2,
            Accepted = 3,
        }
    }

    def_enum! {
        AdvancementTab: VarInt {
            OpenedTab: Ident = 0,
            ClosedScreen = 1,
        }
    }

    def_struct! {
        SelectMerchantTrade {
            selected_slot: VarInt,
        }
    }

    def_struct! {
        UpdateBeacon {
            // TODO: potion ids
            primary_effect: Option<VarInt>,
            secondary_effect: Option<VarInt>,
        }
    }

    def_struct! {
        UpdateSelectedSlot {
            slot: BoundedInt<i16, 0, 8>,
        }
    }

    def_struct! {
        UpdateCommandBlock {
            location: BlockPos,
            command: String,
            mode: CommandBlockMode,
            flags: CommandBlockFlags,
        }
    }

    def_enum! {
        CommandBlockMode: VarInt {
            Sequence = 0,
            Auto = 1,
            Redstone = 2,
        }
    }

    def_bitfield! {
        CommandBlockFlags: i8 {
            track_output = 0,
            is_conditional = 1,
            automatic = 2,
        }
    }

    def_struct! {
        UpdateCommandBlockMinecart {
            entity_id: VarInt,
            command: String,
            track_output: bool,
        }
    }

    def_struct! {
        UpdateCreativeModeSlot {
            slot: i16,
            // TODO: clicked_item: Slot,
        }
    }

    def_struct! {
        UpdateJigsaw {
            location: BlockPos,
            name: Ident,
            target: Ident,
            pool: Ident,
            final_state: String,
            joint_type: String,
        }
    }

    def_struct! {
        UpdateStructureBlock {
            location: BlockPos,
            action: StructureBlockAction,
            mode: StructureBlockMode,
            name: String,
            offset_xyz: [BoundedInt<i8, -32, 32>; 3],
            size_xyz: [BoundedInt<i8, 0, 32>; 3],
            mirror: StructureBlockMirror,
            rotation: StructureBlockRotation,
            metadata: String,
            integrity: f32, // TODO: bounded float between 0 and 1.
            seed: VarLong,
            flags: StructureBlockFlags,
        }
    }

    def_enum! {
        StructureBlockAction: VarInt {
            UpdateData = 0,
            SaveStructure = 1,
            LoadStructure = 2,
            DetectSize = 3,
        }
    }

    def_enum! {
        StructureBlockMode: VarInt {
            Save = 0,
            Load = 1,
            Corner = 2,
            Data = 3,
        }
    }

    def_enum! {
        StructureBlockMirror: VarInt {
            None = 0,
            LeftRight = 1,
            FrontBack = 2,
        }
    }

    def_enum! {
        StructureBlockRotation: VarInt {
            None = 0,
            Clockwise90 = 1,
            Clockwise180 = 2,
            Counterclockwise90 = 3,
        }
    }

    def_bitfield! {
        StructureBlockFlags: i8 {
            ignore_entities = 0,
            show_air = 1,
            show_bounding_box = 2,
        }
    }

    def_struct! {
        UpdateSign {
            location: BlockPos,
            lines: [BoundedString<0, 384>; 4],
        }
    }

    def_struct! {
        HandSwing {
            hand: Hand,
        }
    }

    def_struct! {
        SpectatorTeleport {
            target: Uuid,
        }
    }

    def_struct! {
        PlayerInteractBlock {
            hand: Hand,
            location: BlockPos,
            face: BlockFace,
            cursor_pos: Vec3<f32>,
            head_inside_block: bool,
            sequence: VarInt,
        }
    }

    def_struct! {
        PlayerInteractItem {
            hand: Hand,
            sequence: VarInt,
        }
    }

    def_packet_group! {
        C2sPlayPacket {
            TeleportConfirm = 0,
            QueryBlockNbt = 1,
            UpdateDifficulty = 2,
            MessageAcknowledgment = 3,
            CommandExecution = 4,
            ChatMessage = 5,
            RequestChatPreview = 6,
            ClientStatus = 7,
            ClientSettings = 8,
            RequestCommandCompletion = 9,
            ButtonClick = 10,
            ClickSlot = 11,
            CloseHandledScreen = 12,
            PluginMessageToServer = 13,
            BookUpdate = 14,
            QueryEntityNbt = 15,
            PlayerInteractEntity = 16,
            JigsawGenerate = 17,
            KeepAlive = 18,
            UpdateDifficultyLock = 19,
            MovePlayerPosition = 20,
            MovePlayerPositionAndRotation = 21,
            MovePlayerRotation = 22,
            MovePlayerOnGround = 23,
            MoveVehicle = 24,
            BoatPaddleState = 25,
            PickFromInventory = 26,
            CraftRequest = 27,
            UpdatePlayerAbilities = 28,
            PlayerAction = 29,
            PlayerCommand = 30,
            PlayerInput = 31,
            PlayPong = 32,
            RecipeBookChangeSettings = 33,
            RecipeBookSeenRecipe = 34,
            RenameItem = 35,
            ResourcePackStatus = 36,
            AdvancementTab = 37,
            SelectMerchantTrade = 38,
            UpdateBeacon = 39,
            UpdateSelectedSlot = 40,
            UpdateCommandBlock = 41,
            UpdateCommandBlockMinecart = 42,
            UpdateCreativeModeSlot = 43,
            UpdateJigsaw = 44,
            UpdateStructureBlock = 45,
            UpdateSign = 46,
            HandSwing = 47,
            SpectatorTeleport = 48,
            PlayerInteractBlock = 49,
            PlayerInteractItem = 50,
        }
    }
}
