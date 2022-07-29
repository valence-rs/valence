//! Client to server packets.

use super::*;

pub mod handshake {
    use super::*;

    def_struct! {
        Handshake 0x00 {
            protocol_version: VarInt,
            server_adddress: BoundedString<0, 255>,
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
}

pub mod status {
    use super::*;

    def_struct! {
        QueryRequest 0x00 {}
    }

    def_struct! {
        QueryPing 0x01 {
            payload: u64
        }
    }
}

pub mod login {
    use super::*;

    def_struct! {
        LoginStart 0x00 {
            username: BoundedString<3, 16>,
            sig_data: Option<SignatureData>,
        }
    }

    def_struct! {
        EncryptionResponse 0x01 {
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
        LoginPluginResponse 0x02 {
            message_id: VarInt,
            data: Option<RawBytes>,
        }
    }

    def_packet_group! {
        C2sLoginPacket {
            LoginStart,
            EncryptionResponse,
            LoginPluginResponse,
        }
    }
}

pub mod play {
    use super::super::*;

    def_struct! {
        TeleportConfirm 0x00 {
            teleport_id: VarInt
        }
    }

    def_struct! {
        QueryBlockNbt 0x01 {
            transaction_id: VarInt,
            location: BlockPos,
        }
    }

    def_enum! {
        UpdateDifficulty 0x02: i8 {
            Peaceful = 0,
            Easy = 1,
            Normal = 2,
            Hard = 3,
        }
    }

    def_struct! {
        CommandExecution 0x03 {
            command: String, // TODO: bounded?
            // TODO: timestamp, arg signatures
            signed_preview: bool,
        }
    }

    def_struct! {
        ChatMessage 0x04 {
            message: BoundedString<0, 256>,
            timestamp: u64,
            salt: u64,
            signature: Vec<u8>,
            signed_preview: bool,
        }
    }

    def_struct! {
        RequestChatPreview 0x05 {
            query: i32, // TODO: is this an i32 or a varint?
            message: BoundedString<0, 256>,
        }
    }

    def_enum! {
        ClientStatus 0x06: VarInt {
            /// Sent when ready to complete login and ready to respawn after death.
            PerformRespawn = 0,
            /// Sent when the statistics menu is opened.
            RequestStatus = 1,
        }
    }

    def_struct! {
        ClientSettings 0x07 {
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
        RequestCommandCompletion 0x08 {
            transaction_id: VarInt,
            /// Text behind the cursor without the '/'.
            text: BoundedString<0, 32500>
        }
    }

    def_struct! {
        ButtonClick 0x09 {
            window_id: i8,
            button_id: i8,
        }
    }

    def_struct! {
        ClickSlot 0x0a {
            // TODO
        }
    }

    def_struct! {
        CloseHandledScreen 0x0b {
            window_id: u8,
        }
    }

    def_struct! {
        CustomPayload 0x0c {
            channel: Ident,
            data: RawBytes,
        }
    }

    def_struct! {
        BookUpdate 0x0d {
            slot: VarInt,
            entries: Vec<String>,
            title: Option<String>,
        }
    }

    def_struct! {
        QueryEntityNbt 0x0e {
            transaction_id: VarInt,
            entity_id: VarInt,
        }
    }

    def_struct! {
        PlayerInteractEntity 0x0f {
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
        JigsawGenerate 0x10 {
            location: BlockPos,
            levels: VarInt,
            keep_jigsaws: bool,
        }
    }

    def_struct! {
        KeepAlive 0x11 {
            id: i64,
        }
    }

    def_struct! {
        UpdateDifficultyLock 0x12 {
            locked: bool
        }
    }

    def_struct! {
        MovePlayerPosition 0x13 {
            position: Vec3<f64>,
            on_ground: bool,
        }
    }

    def_struct! {
        MovePlayerPositionAndRotation 0x14 {
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
        MovePlayerRotation 0x15 {
            /// Absolute rotation on X axis in degrees.
            yaw: f32,
            /// Absolute rotation on Y axis in degrees.
            pitch: f32,
            on_ground: bool,
        }
    }

    def_struct! {
        MovePlayerOnGround 0x16 {
            on_ground: bool
        }
    }

    def_struct! {
        MoveVehicle 0x17 {
            /// Absolute position
            position: Vec3<f64>,
            /// Degrees
            yaw: f32,
            /// Degrees
            pitch: f32,
        }
    }

    def_struct! {
        BoatPaddleState 0x18 {
            left_paddle_turning: bool,
            right_paddle_turning: bool,
        }
    }

    def_struct! {
        PickFromInventory 0x19 {
            slot_to_use: VarInt,
        }
    }

    def_struct! {
        CraftRequest 0x1a {
            window_id: i8,
            recipe: Ident,
            make_all: bool,
        }
    }

    def_enum! {
        UpdatePlayerAbilities 0x1b: i8 {
            NotFlying = 0,
            Flying = 0b10,
        }
    }

    def_struct! {
        PlayerAction 0x1c {
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
        PlayerCommand 0x1d {
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
        PlayerInput 0x1e {
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
        PlayPong 0x1f {
            id: i32,
        }
    }

    def_struct! {
        RecipeBookChangeSettings 0x20 {
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
        RecipeBookSeenRecipe 0x21 {
            recipe_id: Ident,
        }
    }

    def_struct! {
        RenameItem 0x22 {
            item_name: BoundedString<0, 50>,
        }
    }

    def_enum! {
        ResourcePackStatus 0x23: VarInt {
            SuccessfullyLoaded = 0,
            Declined = 1,
            FailedDownload = 2,
            Accepted = 3,
        }
    }

    def_enum! {
        AdvancementTab 0x24: VarInt {
            OpenedTab: Ident = 0,
            ClosedScreen = 1,
        }
    }

    def_struct! {
        SelectMerchantTrade 0x25 {
            selected_slot: VarInt,
        }
    }

    def_struct! {
        UpdateBeacon 0x26 {
            // TODO: potion ids
            primary_effect: Option<VarInt>,
            secondary_effect: Option<VarInt>,
        }
    }

    def_struct! {
        UpdateSelectedSlot 0x27 {
            slot: BoundedInt<i16, 0, 8>,
        }
    }

    def_struct! {
        UpdateCommandBlock 0x28 {
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
        UpdateCommandBlockMinecart 0x29 {
            entity_id: VarInt,
            command: String,
            track_output: bool,
        }
    }

    def_struct! {
        UpdateCreativeModeSlot 0x2a {
            slot: i16,
            // TODO: clicked_item: Slot,
        }
    }

    def_struct! {
        UpdateJigsaw 0x2b {
            location: BlockPos,
            name: Ident,
            target: Ident,
            pool: Ident,
            final_state: String,
            joint_type: String,
        }
    }

    def_struct! {
        UpdateStructureBlock 0x2c {
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
        UpdateSign 0x2d {
            location: BlockPos,
            lines: [BoundedString<0, 384>; 4],
        }
    }

    def_struct! {
        HandSwing 0x2e {
            hand: Hand,
        }
    }

    def_struct! {
        SpectatorTeleport 0x2f {
            target: Uuid,
        }
    }

    def_struct! {
        PlayerInteractBlock 0x30 {
            hand: Hand,
            location: BlockPos,
            face: BlockFace,
            cursor_pos: Vec3<f32>,
            head_inside_block: bool,
            sequence: VarInt,
        }
    }

    def_struct! {
        PlayerInteractItem 0x31 {
            hand: Hand,
            sequence: VarInt,
        }
    }

    def_packet_group! {
        C2sPlayPacket {
            TeleportConfirm,
            QueryBlockNbt,
            UpdateDifficulty,
            CommandExecution,
            ChatMessage,
            RequestChatPreview,
            ClientStatus,
            ClientSettings,
            RequestCommandCompletion,
            ButtonClick,
            ClickSlot,
            CloseHandledScreen,
            CustomPayload,
            BookUpdate,
            QueryEntityNbt,
            PlayerInteractEntity,
            JigsawGenerate,
            KeepAlive,
            UpdateDifficultyLock,
            MovePlayerPosition,
            MovePlayerPositionAndRotation,
            MovePlayerRotation,
            MovePlayerOnGround,
            MoveVehicle,
            BoatPaddleState,
            PickFromInventory,
            CraftRequest,
            UpdatePlayerAbilities,
            PlayerAction,
            PlayerCommand,
            PlayerInput,
            PlayPong,
            RecipeBookChangeSettings,
            RecipeBookSeenRecipe,
            RenameItem,
            ResourcePackStatus,
            AdvancementTab,
            SelectMerchantTrade,
            UpdateBeacon,
            UpdateSelectedSlot,
            UpdateCommandBlock,
            UpdateCommandBlockMinecart,
            UpdateCreativeModeSlot,
            UpdateJigsaw,
            UpdateStructureBlock,
            UpdateSign,
            HandSwing,
            SpectatorTeleport,
            PlayerInteractBlock,
            PlayerInteractItem,
        }
    }
}
