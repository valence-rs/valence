use uuid::Uuid;

use crate::block::BlockFace;
use crate::block_pos::BlockPos;
use crate::ident::Ident;
use crate::item::ItemStack;
use crate::raw_bytes::RawBytes;
use crate::types::{
    Action, ChatMode, ClickContainerMode, CommandArgumentSignature, CommandBlockFlags,
    CommandBlockMode, DiggingStatus, DisplayedSkinParts, EntityInteraction, Hand,
    HandshakeNextState, MainHand, MessageAcknowledgment, MsgSigOrVerifyToken, PlayerInputFlags,
    PublicKeyData, RecipeBookId, StructureBlockAction, StructureBlockFlags, StructureBlockMirror,
    StructureBlockMode, StructureBlockRotation,
};
use crate::username::Username;
use crate::var_int::VarInt;
use crate::var_long::VarLong;
use crate::{Decode, Encode, Packet};

pub mod handshake {
    use super::*;

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x00]
    pub struct Handshake<'a> {
        pub protocol_version: VarInt,
        pub server_address: &'a str,
        pub server_port: u16,
        pub next_state: HandshakeNextState,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x00]
    pub struct HandshakeOwned {
        pub protocol_version: VarInt,
        pub server_address: String,
        pub server_port: u16,
        pub next_state: HandshakeNextState,
    }

    packet_enum! {
        #[derive(Clone, Debug)]
        C2sHandshakePacket<'a> {
            Handshake<'a>
        }
    }
}

pub mod status {
    use super::*;

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x00]
    pub struct StatusRequest;

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x01]
    pub struct PingRequest {
        pub payload: u64,
    }

    packet_enum! {
        #[derive(Clone, Debug)]
        C2sStatusPacket {
            StatusRequest,
            PingRequest,
        }
    }
}

pub mod login {
    use super::*;

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x00]
    pub struct LoginStart<'a> {
        pub username: Username<&'a str>,
        pub sig_data: Option<PublicKeyData<'a>>,
        pub profile_id: Option<Uuid>,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x01]
    pub struct EncryptionResponse<'a> {
        pub shared_secret: &'a [u8],
        pub sig_or_token: MsgSigOrVerifyToken<'a>,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x02]
    pub struct LoginPluginResponse<'a> {
        pub message_id: VarInt,
        pub data: Option<RawBytes<'a>>,
    }

    packet_enum! {
        #[derive(Clone, Debug)]
        C2sLoginPacket<'a> {
            LoginStart<'a>,
            EncryptionResponse<'a>,
            LoginPluginResponse<'a>,
        }
    }
}

pub mod play {
    use super::*;

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x00]
    pub struct ConfirmTeleport {
        pub teleport_id: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x01]
    pub struct QueryBlockEntityTag {
        pub transaction_id: VarInt,
        pub location: BlockPos,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x02]
    pub enum ChangeDifficulty {
        Peaceful,
        Easy,
        Normal,
        Hard,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x03]
    pub struct MessageAcknowledgmentC2s<'a>(pub MessageAcknowledgment<'a>);

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x04]
    pub struct ChatCommand<'a> {
        pub command: &'a str,
        pub timestamp: u64,
        pub salt: u64,
        pub argument_signatures: Vec<CommandArgumentSignature<'a>>,
        pub signed_preview: bool,
        pub acknowledgement: MessageAcknowledgment<'a>,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x05]
    pub struct ChatMessage<'a> {
        pub message: &'a str,
        pub timestamp: u64,
        pub salt: u64,
        pub signature: &'a [u8],
        pub signed_preview: bool,
        pub acknowledgement: MessageAcknowledgment<'a>,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x06]
    pub struct ChatPreviewC2s {
        // TODO
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x07]
    pub enum ClientCommand {
        PerformRespawn,
        RequestStatus,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x08]
    pub struct ClientInformation<'a> {
        pub locale: &'a str,
        pub view_distance: u8,
        pub chat_mode: ChatMode,
        pub chat_colors: bool,
        pub displayed_skin_parts: DisplayedSkinParts,
        pub main_hand: MainHand,
        pub enable_text_filtering: bool,
        pub allow_server_listings: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x09]
    pub struct CommandSuggestionsRequest<'a> {
        pub transaction_id: VarInt,
        pub text: &'a str,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x0a]
    pub struct ClickContainerButton {
        pub window_id: i8,
        pub button_id: i8,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x0b]
    pub struct ClickContainer {
        pub window_id: u8,
        pub state_id: VarInt,
        pub slot_idx: i16,
        pub button: i8,
        pub mode: ClickContainerMode,
        pub slots: Vec<(i16, Option<ItemStack>)>,
        pub carried_item: Option<ItemStack>,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x0c]
    pub struct CloseContainerC2s {
        pub window_id: u8,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x0d]
    pub struct PluginMessageC2s<'a> {
        pub channel: Ident<&'a str>,
        pub data: RawBytes<'a>,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x0e]
    pub struct EditBook<'a> {
        pub slot: VarInt,
        pub entries: Vec<&'a str>,
        pub title: Option<&'a str>,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x0f]
    pub struct QueryEntityTag {
        pub transaction_id: VarInt,
        pub entity_id: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x10]
    pub struct Interact {
        pub entity_id: VarInt,
        pub interact: EntityInteraction,
        pub sneaking: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x11]
    pub struct JigsawGenerate {
        pub location: BlockPos,
        pub levels: VarInt,
        pub keep_jigsaws: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x12]
    pub struct KeepAliveC2s {
        pub id: u64,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x13]
    pub struct LockDifficulty {
        pub locked: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x14]
    pub struct SetPlayerPosition {
        pub position: [f64; 3],
        pub on_ground: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x15]
    pub struct SetPlayerPositionAndRotation {
        pub position: [f64; 3],
        pub yaw: f32,
        pub pitch: f32,
        pub on_ground: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x16]
    pub struct SetPlayerRotation {
        pub yaw: f32,
        pub pitch: f32,
        pub on_ground: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x17]
    pub struct SetPlayerOnGround(pub bool);

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x18]
    pub struct MoveVehicleC2s {
        pub position: [f64; 3],
        pub yaw: f32,
        pub pitch: f32,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x19]
    pub struct PaddleBoat {
        pub left_paddle_turning: bool,
        pub right_paddle_turning: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x1a]
    pub struct PickItem {
        pub slot_to_use: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x1b]
    pub struct PlaceRecipe<'a> {
        pub window_id: i8,
        pub recipe: Ident<&'a str>,
        pub make_all: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x1c]
    pub enum PlayerAbilitiesC2s {
        #[tag = 0b00]
        StopFlying,
        #[tag = 0b10]
        StartFlying,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x1d]
    pub struct PlayerAction {
        pub status: DiggingStatus,
        pub location: BlockPos,
        pub face: BlockFace,
        pub sequence: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x1e]
    pub struct PlayerCommand {
        pub entity_id: VarInt,
        pub action_id: Action,
        pub jump_boost: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x1f]
    pub struct PlayerInput {
        pub sideways: f32,
        pub forward: f32,
        pub flags: PlayerInputFlags,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x20]
    pub struct PongPlay {
        pub id: i32,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x21]
    pub struct ChangeRecipeBookSettings {
        pub book_id: RecipeBookId,
        pub book_open: bool,
        pub filter_active: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x22]
    pub struct SetSeenRecipe<'a> {
        pub recipe_id: Ident<&'a str>,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x23]
    pub struct RenameItem<'a> {
        pub item_name: &'a str,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x24]
    pub enum ResourcePackC2s {
        SuccessfullyLoaded,
        Declined,
        FailedDownload,
        Accepted,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x25]
    pub enum SeenAdvancements<'a> {
        OpenedTab { tab_id: Ident<&'a str> },
        ClosedScreen,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x26]
    pub struct SelectTrade {
        pub selected_slot: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x27]
    pub struct SetBeaconEffect {
        pub primary_effect: Option<VarInt>,
        pub secondary_effect: Option<VarInt>,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x28]
    pub struct SetHeldItemC2s {
        pub slot: i16,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x29]
    pub struct ProgramCommandBlock<'a> {
        pub location: BlockPos,
        pub command: &'a str,
        pub mode: CommandBlockMode,
        pub flags: CommandBlockFlags,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x2a]
    pub struct ProgramCommandBlockMinecart<'a> {
        pub entity_id: VarInt,
        pub command: &'a str,
        pub track_output: bool,
    }

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x2b]
    pub struct SetCreativeModeSlot {
        pub slot: i16,
        pub clicked_item: Option<ItemStack>,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x2c]
    pub struct ProgramJigsawBlock<'a> {
        pub location: BlockPos,
        pub name: Ident<&'a str>,
        pub target: Ident<&'a str>,
        pub pool: Ident<&'a str>,
        pub final_state: &'a str,
        pub joint_type: &'a str,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x2d]
    pub struct ProgramStructureBlock<'a> {
        pub location: BlockPos,
        pub action: StructureBlockAction,
        pub mode: StructureBlockMode,
        pub name: &'a str,
        pub offset_xyz: [i8; 3],
        pub size_xyz: [i8; 3],
        pub mirror: StructureBlockMirror,
        pub rotation: StructureBlockRotation,
        pub metadata: &'a str,
        pub integrity: f32,
        pub seed: VarLong,
        pub flags: StructureBlockFlags,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x2e]
    pub struct UpdateSign<'a> {
        pub location: BlockPos,
        pub lines: [&'a str; 4],
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x2f]
    pub struct SwingArm(pub Hand);

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x30]
    pub struct TeleportToEntity {
        pub target: Uuid,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x31]
    pub struct UseItemOn {
        pub hand: Hand,
        pub location: BlockPos,
        pub face: BlockFace,
        pub cursor_pos: [f32; 3],
        pub head_inside_block: bool,
        pub sequence: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
    #[packet_id = 0x32]
    pub struct UseItem {
        pub hand: Hand,
        pub sequence: VarInt,
    }

    packet_enum! {
        #[derive(Clone, Debug)]
        C2sPlayPacket<'a> {
            ConfirmTeleport,
            QueryBlockEntityTag,
            ChangeDifficulty,
            MessageAcknowledgmentC2s<'a>,
            ChatCommand<'a>,
            ChatMessage<'a>,
            ChatPreviewC2s,
            ClientCommand,
            ClientInformation<'a>,
            CommandSuggestionsRequest<'a>,
            ClickContainerButton,
            ClickContainer,
            CloseContainerC2s,
            PluginMessageC2s<'a>,
            EditBook<'a>,
            QueryEntityTag,
            Interact,
            JigsawGenerate,
            KeepAliveC2s,
            LockDifficulty,
            SetPlayerPosition,
            SetPlayerPositionAndRotation,
            SetPlayerRotation,
            SetPlayerOnGround,
            MoveVehicleC2s,
            PaddleBoat,
            PickItem,
            PlaceRecipe<'a>,
            PlayerAbilitiesC2s,
            PlayerAction,
            PlayerCommand,
            PlayerInput,
            PongPlay,
            ChangeRecipeBookSettings,
            SetSeenRecipe<'a>,
            RenameItem<'a>,
            ResourcePackC2s,
            SeenAdvancements<'a>,
            SelectTrade,
            SetBeaconEffect,
            SetHeldItemC2s,
            ProgramCommandBlock<'a>,
            ProgramCommandBlockMinecart<'a>,
            SetCreativeModeSlot,
            ProgramJigsawBlock<'a>,
            ProgramStructureBlock<'a>,
            UpdateSign<'a>,
            SwingArm,
            TeleportToEntity,
            UseItemOn,
            UseItem
        }
    }
}
