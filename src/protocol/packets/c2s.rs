//! Client to server packets.

use super::*;

pub mod handshake {
    use super::*;

    def_struct! {
        Handshake {
            protocol_version: VarInt,
            server_address: BoundedString<0, 255>,
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
}

pub mod status {
    use super::*;

    def_struct! {
        StatusRequest {}
    }

    def_struct! {
        PingRequest {
            payload: u64
        }
    }

    def_packet_group! {
        C2sStatusPacket {
            StatusRequest = 0,
            PingRequest = 1,
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
    use super::*;

    def_struct! {
        ConfirmTeleport {
            teleport_id: VarInt
        }
    }

    def_struct! {
        QueryBlockEntityTag {
            transaction_id: VarInt,
            location: BlockPos,
        }
    }

    def_enum! {
        ChangeDifficulty: i8 {
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
        ChatCommand {
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
        ChatPreviewC2s {
            query: i32, // TODO: is this an i32 or a varint?
            message: BoundedString<0, 256>,
        }
    }

    def_enum! {
        ClientCommand: VarInt {
            /// Sent when ready to complete login and ready to respawn after death.
            PerformRespawn = 0,
            /// Sent when the statistics menu is opened.
            RequestStatus = 1,
        }
    }

    def_struct! {
        ClientInformation {
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
        CommandSuggestionsRequest {
            transaction_id: VarInt,
            /// Text behind the cursor without the '/'.
            text: BoundedString<0, 32500>
        }
    }

    def_struct! {
        ClickContainerButton {
            window_id: i8,
            button_id: i8,
        }
    }

    def_struct! {
        ClickContainer {
            window_id: u8,
            state_id: VarInt,
            slot_idx: i16,
            button: i8,
            mode: ClickContainerMode,
            slots: Vec<(i16, Slot)>,
            carried_item: Slot,
        }
    }

    def_enum! {
        #[derive(Copy, PartialEq, Eq)]
        ClickContainerMode: VarInt {
            Click = 0,
            ShiftClick = 1,
            Hotbar = 2,
            CreativeMiddleClick = 3,
            DropKey = 4,
            Drag = 5,
            DoubleClick = 6,
        }
    }

    def_struct! {
        CloseContainerC2s {
            window_id: u8,
        }
    }

    def_struct! {
        PluginMessageC2s {
            channel: Ident<'static>,
            data: RawBytes,
        }
    }

    def_struct! {
        EditBook {
            slot: VarInt,
            entries: Vec<String>,
            title: Option<String>,
        }
    }

    def_struct! {
        QueryEntityTag {
            transaction_id: VarInt,
            entity_id: VarInt,
        }
    }

    def_struct! {
        Interact {
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
        KeepAliveC2s {
            id: i64,
        }
    }

    def_struct! {
        LockDifficulty {
            locked: bool
        }
    }

    def_struct! {
        SetPlayerPosition {
            position: Vec3<f64>,
            on_ground: bool,
        }
    }

    def_struct! {
        SetPlayerPositionAndRotation {
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
        SetPlayerRotation {
            /// Absolute rotation on X axis in degrees.
            yaw: f32,
            /// Absolute rotation on Y axis in degrees.
            pitch: f32,
            on_ground: bool,
        }
    }

    def_struct! {
        SetPlayerOnGround {
            on_ground: bool
        }
    }

    def_struct! {
        MoveVehicleC2s {
            /// Absolute position
            position: Vec3<f64>,
            /// Degrees
            yaw: f32,
            /// Degrees
            pitch: f32,
        }
    }

    def_struct! {
        PaddleBoat {
            left_paddle_turning: bool,
            right_paddle_turning: bool,
        }
    }

    def_struct! {
        PickItem {
            slot_to_use: VarInt,
        }
    }

    def_struct! {
        PlaceRecipe {
            window_id: i8,
            recipe: Ident<'static>,
            make_all: bool,
        }
    }

    def_enum! {
        PlayerAbilitiesC2s: i8 {
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
        PongPlay {
            id: i32,
        }
    }

    def_struct! {
        ChangeRecipeBookSettings {
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
        SetSeenRecipe {
            recipe_id: Ident<'static>,
        }
    }

    def_struct! {
        RenameItem {
            item_name: BoundedString<0, 50>,
        }
    }

    def_enum! {
        #[derive(Copy, PartialEq, Eq)]
        ResourcePackC2s: VarInt {
            SuccessfullyLoaded = 0,
            Declined = 1,
            FailedDownload = 2,
            Accepted = 3,
        }
    }

    def_enum! {
        SeenAdvancements: VarInt {
            OpenedTab: Ident<'static> = 0,
            ClosedScreen = 1,
        }
    }

    def_struct! {
        SelectTrade {
            selected_slot: VarInt,
        }
    }

    def_struct! {
        SetBeaconEffect {
            // TODO: potion ids
            primary_effect: Option<VarInt>,
            secondary_effect: Option<VarInt>,
        }
    }

    def_struct! {
        SetHeldItemS2c {
            slot: BoundedInt<i16, 0, 8>,
        }
    }

    def_struct! {
        ProgramCommandBlock {
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
        ProgramCommandBlockMinecart {
            entity_id: VarInt,
            command: String,
            track_output: bool,
        }
    }

    def_struct! {
        SetCreativeModeSlot {
            slot: i16,
            clicked_item: Slot,
        }
    }

    def_struct! {
        ProgramJigsawBlock {
            location: BlockPos,
            name: Ident<'static>,
            target: Ident<'static>,
            pool: Ident<'static>,
            final_state: String,
            joint_type: String,
        }
    }

    def_struct! {
        ProgramStructureBlock {
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
        SwingArm {
            hand: Hand,
        }
    }

    def_struct! {
        TeleportToEntity {
            target: Uuid,
        }
    }

    def_struct! {
        UseItemOn {
            hand: Hand,
            location: BlockPos,
            face: BlockFace,
            cursor_pos: Vec3<f32>,
            head_inside_block: bool,
            sequence: VarInt,
        }
    }

    def_struct! {
        UseItem {
            hand: Hand,
            sequence: VarInt,
        }
    }

    def_packet_group! {
        C2sPlayPacket {
            ConfirmTeleport = 0,
            QueryBlockEntityTag = 1,
            ChangeDifficulty = 2,
            MessageAcknowledgment = 3,
            ChatCommand = 4,
            ChatMessage = 5,
            ChatPreviewC2s = 6,
            ClientCommand = 7,
            ClientInformation = 8,
            CommandSuggestionsRequest = 9,
            ClickContainerButton = 10,
            ClickContainer = 11,
            CloseContainerC2s = 12,
            PluginMessageC2s = 13,
            EditBook = 14,
            QueryEntityTag = 15,
            Interact = 16,
            JigsawGenerate = 17,
            KeepAliveC2s = 18,
            LockDifficulty = 19,
            SetPlayerPosition = 20,
            SetPlayerPositionAndRotation = 21,
            SetPlayerRotation = 22,
            SetPlayerOnGround = 23,
            MoveVehicleC2s = 24,
            PaddleBoat = 25,
            PickItem = 26,
            PlaceRecipe = 27,
            PlayerAbilitiesC2s = 28,
            PlayerAction = 29,
            PlayerCommand = 30,
            PlayerInput = 31,
            PongPlay = 32,
            ChangeRecipeBookSettings = 33,
            SetSeenRecipe = 34,
            RenameItem = 35,
            ResourcePackC2s = 36,
            SeenAdvancements = 37,
            SelectTrade = 38,
            SetBeaconEffect = 39,
            SetHeldItemS2c = 40,
            ProgramCommandBlock = 41,
            ProgramCommandBlockMinecart = 42,
            SetCreativeModeSlot = 43,
            ProgramJigsawBlock = 44,
            ProgramStructureBlock = 45,
            UpdateSign = 46,
            SwingArm = 47,
            TeleportToEntity = 48,
            UseItemOn = 49,
            UseItem = 50,
        }
    }
}
