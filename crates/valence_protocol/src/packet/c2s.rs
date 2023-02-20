use uuid::Uuid;

use crate::block::BlockFace;
use crate::block_pos::BlockPos;
use crate::ident::Ident;
use crate::item::ItemStack;
use crate::raw_bytes::RawBytes;
use crate::types::{
    Action, ChatMode, ClickContainerMode, CommandArgumentSignature, CommandBlockFlags,
    CommandBlockMode, Difficulty, DiggingStatus, DisplayedSkinParts, EntityInteraction, Hand,
    HandshakeNextState, MainHand, PlayerInputFlags, RecipeBookId, StructureBlockAction,
    StructureBlockFlags, StructureBlockMirror, StructureBlockMode, StructureBlockRotation,
};
use crate::username::Username;
use crate::var_int::VarInt;
use crate::var_long::VarLong;
use crate::{Decode, DecodePacket, Encode, EncodePacket};

pub mod handshake {
    use super::*;

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x00]
    pub struct HandshakeC2s<'a> {
        pub protocol_version: VarInt,
        pub server_address: &'a str,
        pub server_port: u16,
        pub next_state: HandshakeNextState,
    }

    packet_enum! {
        #[derive(Clone)]
        C2sHandshakePacket<'a> {
            HandshakeC2s<'a>
        }
    }
}

pub mod status {
    use super::*;

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x00]
    pub struct QueryRequestC2s;

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x01]
    pub struct QueryPingC2s {
        pub payload: u64,
    }

    packet_enum! {
        #[derive(Clone)]
        C2sStatusPacket {
            QueryRequestC2s,
            QueryPingC2s,
        }
    }
}

pub mod login {
    use super::*;

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x00]
    pub struct LoginHelloC2s<'a> {
        pub username: Username<&'a str>,
        pub profile_id: Option<Uuid>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x01]
    pub struct LoginKeyC2s<'a> {
        pub shared_secret: &'a [u8],
        pub verify_token: &'a [u8],
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x02]
    pub struct LoginQueryResponseC2s<'a> {
        pub message_id: VarInt,
        pub data: Option<RawBytes<'a>>,
    }

    packet_enum! {
        #[derive(Clone)]
        C2sLoginPacket<'a> {
            LoginHelloC2s<'a>,
            LoginKeyC2s<'a>,
            LoginQueryResponseC2s<'a>,
        }
    }
}

pub mod play {
    use super::*;

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x00]
    pub struct TeleportConfirmC2s {
        pub teleport_id: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x01]
    pub struct QueryBlockNbtC2s {
        pub transaction_id: VarInt,
        pub position: BlockPos,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x02]
    pub struct UpdateDifficultyC2s {
        pub new_difficulty: Difficulty,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x03]
    pub struct MessageAcknowledgmentC2s {
        pub message_count: VarInt,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x04]
    pub struct CommandExecutionC2s<'a> {
        pub command: &'a str,
        pub timestamp: u64,
        pub salt: u64,
        pub argument_signatures: Vec<CommandArgumentSignature<'a>>,
        pub message_count: VarInt,
        // This is a bitset of 20; each bit represents one
        // of the last 20 messages received and whether or not
        // the message was acknowledged by the client
        pub acknowledgement: &'a [u8; 3],
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x05]
    pub struct ChatMessageC2s<'a> {
        pub message: &'a str,
        pub timestamp: u64,
        pub salt: u64,
        pub signature: Option<&'a [u8; 256]>,
        pub message_count: VarInt,
        // This is a bitset of 20; each bit represents one
        // of the last 20 messages received and whether or not
        // the message was acknowledged by the client
        pub acknowledgement: &'a [u8; 3],
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x06]
    pub enum ClientStatusC2s {
        PerformRespawn,
        RequestStats,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x07]
    pub struct ClientSettingsC2s<'a> {
        pub locale: &'a str,
        pub view_distance: u8,
        pub chat_mode: ChatMode,
        pub chat_colors: bool,
        pub displayed_skin_parts: DisplayedSkinParts,
        pub main_hand: MainHand,
        pub enable_text_filtering: bool,
        pub allow_server_listings: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x08]
    pub struct RequestCommandCompletionsC2s<'a> {
        pub transaction_id: VarInt,
        pub text: &'a str,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x09]
    pub struct ButtonClickC2s {
        pub window_id: i8,
        pub button_id: i8,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x0a]
    pub struct ClickSlotC2s {
        pub window_id: u8,
        pub state_id: VarInt,
        pub slot_idx: i16,
        pub button: i8,
        pub mode: ClickContainerMode,
        pub slots: Vec<(i16, Option<ItemStack>)>,
        pub carried_item: Option<ItemStack>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x0b]
    pub struct CloseHandledScreenC2s {
        pub window_id: i8,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x0c]
    pub struct CustomPayloadC2s<'a> {
        pub channel: Ident<&'a str>,
        pub data: RawBytes<'a>,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x0d]
    pub struct BookUpdateC2s<'a> {
        pub slot: VarInt,
        pub entries: Vec<&'a str>,
        pub title: Option<&'a str>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x0e]
    pub struct QueryEntityNbtC2s {
        pub transaction_id: VarInt,
        pub entity_id: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x0f]
    pub struct PlayerInteractC2s {
        pub entity_id: VarInt,
        pub interact: EntityInteraction,
        pub sneaking: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x10]
    pub struct JigsawGeneratingC2s {
        pub position: BlockPos,
        pub levels: VarInt,
        pub keep_jigsaws: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x11]
    pub struct KeepAliveC2s {
        pub id: u64,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x12]
    pub struct UpdateDifficultyLockC2s {
        pub locked: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x13]
    pub struct PositionAndOnGroundC2s {
        pub position: [f64; 3],
        pub on_ground: bool,
    }

    // TODO: move to position module.
    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x14]
    pub struct FullC2s {
        pub position: [f64; 3],
        pub yaw: f32,
        pub pitch: f32,
        pub on_ground: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x15]
    pub struct LookAndOnGroundC2s {
        pub yaw: f32,
        pub pitch: f32,
        pub on_ground: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x16]
    pub struct OnGroundOnlyC2s {
        pub on_ground: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x17]
    pub struct VehicleMoveC2s {
        pub position: [f64; 3],
        pub yaw: f32,
        pub pitch: f32,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x18]
    pub struct BoatPaddleStateC2s {
        pub left_paddle_turning: bool,
        pub right_paddle_turning: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x19]
    pub struct PickFromInventoryC2s {
        pub slot_to_use: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x1a]
    pub struct CraftRequestC2s<'a> {
        pub window_id: i8,
        pub recipe: Ident<&'a str>,
        pub make_all: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x1b]
    pub enum UpdatePlayerAbilitiesC2s {
        #[tag = 0b00]
        StopFlying,
        #[tag = 0b10]
        StartFlying,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x1c]
    pub struct PlayerActionC2s {
        pub status: DiggingStatus,
        pub position: BlockPos,
        pub face: BlockFace,
        pub sequence: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x1d]
    pub struct ClientCommandC2s {
        pub entity_id: VarInt,
        pub action_id: Action,
        pub jump_boost: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x1e]
    pub struct PlayerInputC2s {
        pub sideways: f32,
        pub forward: f32,
        pub flags: PlayerInputFlags,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x1f]
    pub struct PlayPongC2s {
        pub id: i32,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x20]
    pub struct PlayerSessionC2s<'a> {
        pub session_id: Uuid,
        // Public key
        pub expires_at: i64,
        pub public_key_data: &'a [u8],
        pub key_signature: &'a [u8],
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x21]
    pub struct RecipeCategoryOptionsC2s {
        pub book_id: RecipeBookId,
        pub book_open: bool,
        pub filter_active: bool,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x22]
    pub struct RecipeBookDataC2s<'a> {
        pub recipe_id: Ident<&'a str>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x23]
    pub struct RenameItemC2s<'a> {
        pub item_name: &'a str,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x24]
    pub enum ResourcePackStatusC2s {
        SuccessfullyLoaded,
        Declined,
        FailedDownload,
        Accepted,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x25]
    pub enum AdvancementTabC2s<'a> {
        OpenedTab { tab_id: Ident<&'a str> },
        ClosedScreen,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x26]
    pub struct SelectMerchantTradeC2s {
        pub selected_slot: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x27]
    pub struct UpdateBeaconC2s {
        pub primary_effect: Option<VarInt>,
        pub secondary_effect: Option<VarInt>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x28]
    pub struct UpdateSelectedSlotC2s {
        pub slot: i16,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x29]
    pub struct UpdateCommandBlockC2s<'a> {
        pub position: BlockPos,
        pub command: &'a str,
        pub mode: CommandBlockMode,
        pub flags: CommandBlockFlags,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x2a]
    pub struct UpdateCommandBlockMinecartC2s<'a> {
        pub entity_id: VarInt,
        pub command: &'a str,
        pub track_output: bool,
    }

    #[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x2b]
    pub struct CreativeInventoryActionC2s {
        pub slot: i16,
        pub clicked_item: Option<ItemStack>,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x2c]
    pub struct UpdateJigsawC2s<'a> {
        pub position: BlockPos,
        pub name: Ident<&'a str>,
        pub target: Ident<&'a str>,
        pub pool: Ident<&'a str>,
        pub final_state: &'a str,
        pub joint_type: &'a str,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x2d]
    pub struct UpdateStructureBlockC2s<'a> {
        pub position: BlockPos,
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

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x2e]
    pub struct UpdateSignC2s<'a> {
        pub position: BlockPos,
        pub lines: [&'a str; 4],
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x2f]
    pub struct HandSwingC2s {
        pub hand: Hand,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x30]
    pub struct SpectatorTeleportC2s {
        pub target: Uuid,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x31]
    pub struct PlayerInteractBlockC2s {
        pub hand: Hand,
        pub position: BlockPos,
        pub face: BlockFace,
        pub cursor_pos: [f32; 3],
        pub head_inside_block: bool,
        pub sequence: VarInt,
    }

    #[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
    #[packet_id = 0x32]
    pub struct PlayerInteractItemC2s {
        pub hand: Hand,
        pub sequence: VarInt,
    }

    packet_enum! {
        #[derive(Clone)]
        C2sPlayPacket<'a> {
            TeleportConfirmC2s,
            QueryBlockNbtC2s,
            UpdateDifficultyC2s,
            MessageAcknowledgmentC2s,
            CommandExecutionC2s<'a>,
            ChatMessageC2s<'a>,
            ClientStatusC2s,
            ClientSettingsC2s<'a>,
            RequestCommandCompletionsC2s<'a>,
            ButtonClickC2s,
            ClickSlotC2s,
            CloseHandledScreenC2s,
            CustomPayloadC2s<'a>,
            BookUpdateC2s<'a>,
            QueryEntityNbtC2s,
            PlayerInteractC2s,
            JigsawGeneratingC2s,
            KeepAliveC2s,
            UpdateDifficultyLockC2s,
            PositionAndOnGroundC2s,
            FullC2s,
            LookAndOnGroundC2s,
            OnGroundOnlyC2s,
            VehicleMoveC2s,
            BoatPaddleStateC2s,
            PickFromInventoryC2s,
            CraftRequestC2s<'a>,
            UpdatePlayerAbilitiesC2s,
            PlayerActionC2s,
            ClientCommandC2s,
            PlayerInputC2s,
            PlayPongC2s,
            PlayerSessionC2s<'a>,
            RecipeCategoryOptionsC2s,
            RecipeBookDataC2s<'a>,
            RenameItemC2s<'a>,
            ResourcePackStatusC2s,
            AdvancementTabC2s<'a>,
            SelectMerchantTradeC2s,
            UpdateBeaconC2s,
            UpdateSelectedSlotC2s,
            UpdateCommandBlockC2s<'a>,
            UpdateCommandBlockMinecartC2s<'a>,
            CreativeInventoryActionC2s,
            UpdateJigsawC2s<'a>,
            UpdateStructureBlockC2s<'a>,
            UpdateSignC2s<'a>,
            HandSwingC2s,
            SpectatorTeleportC2s,
            PlayerInteractBlockC2s,
            PlayerInteractItemC2s
        }
    }
}
