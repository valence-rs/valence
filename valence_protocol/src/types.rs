//! Miscellaneous type definitions used in packets.

use uuid::Uuid;
use valence_nbt::Compound;
use valence_protocol::text::Text;

use crate::__private::VarInt;
use crate::block_pos::BlockPos;
use crate::ident::Ident;
use crate::username::Username;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub enum HandshakeNextState {
    #[tag = 1]
    Status,
    #[tag = 2]
    Login,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub struct PublicKeyData<'a> {
    pub timestamp: u64,
    pub public_key: &'a [u8],
    pub signature: &'a [u8],
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub enum MsgSigOrVerifyToken<'a> {
    MsgSig { salt: u64, sig: &'a [u8] },
    VerifyToken(&'a [u8]),
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct MessageAcknowledgment<'a> {
    pub last_seen: Vec<MessageAcknowledgmentEntry<'a>>,
    pub last_received: Option<MessageAcknowledgmentEntry<'a>>,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct MessageAcknowledgmentEntry<'a> {
    pub profile_id: Uuid,
    pub signature: &'a [u8],
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct CommandArgumentSignature<'a> {
    pub argument_name: &'a str,
    pub signature: &'a [u8],
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum ChatMode {
    Enabled,
    CommandsOnly,
    Hidden,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum MainHand {
    Left,
    Right,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub enum ClickContainerMode {
    Click,
    ShiftClick,
    Hotbar,
    CreativeMiddleClick,
    DropKey,
    Drag,
    DoubleClick,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum Hand {
    Main,
    Off,
}

#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode)]
pub enum EntityInteraction {
    Interact {
        hand: Hand,
        sneaking: bool,
    },
    Attack {
        sneaking: bool,
    },
    InteractAt {
        target: [f32; 3],
        hand: Hand,
        sneaking: bool,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum DiggingStatus {
    StartedDigging,
    CancelledDigging,
    FinishedDigging,
    DropItemStack,
    DropItem,
    ShootArrowOrFinishEating,
    SwapItemInHand,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum Action {
    StartSneaking,
    StopSneaking,
    LeaveBed,
    StartSprinting,
    StopSprinting,
    StartJumpWithHorse,
    StopJumpWithHorse,
    OpenHorseInventory,
    StartFlyingWithElytra,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum RecipeBookId {
    Crafting,
    Furnace,
    BlastFurnace,
    Smoker,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum CommandBlockMode {
    Sequence,
    Auto,
    Redstone,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum StructureBlockAction {
    UpdateData,
    SaveStructure,
    LoadStructure,
    DetectSize,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum StructureBlockMode {
    Save,
    Load,
    Corner,
    Data,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum StructureBlockMirror {
    None,
    LeftRight,
    FrontBack,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum StructureBlockRotation {
    None,
    Clockwise90,
    Clockwise180,
    Counterclockwise90,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub struct SignedProperty<'a> {
    pub name: &'a str,
    pub value: &'a str,
    pub signature: Option<&'a str>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum Animation {
    SwingMainArm,
    TakeDamage,
    LeaveBed,
    SwingOffhand,
    CriticalEffect,
    MagicCriticalEffect,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub enum BossBarAction {
    Add {
        title: Text,
        health: f32,
        color: BossBarColor,
        division: BossBarDivision,
        // TODO: BossBarFlags bitfield.
        flags: u8,
    },
    Remove,
    UpdateHealth(f32),
    UpdateTitle(Text),
    UpdateStyle(BossBarColor, BossBarDivision),
    // TODO: BossBarFlags
    UpdateFlags(u8),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum BossBarColor {
    Pink,
    Blue,
    Red,
    Green,
    Yellow,
    Purple,
    White,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum BossBarDivision {
    NoDivision,
    SixNotches,
    TenNotches,
    TwelveNotches,
    TwentyNotches,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum SoundCategory {
    Master,
    Music,
    Record,
    Weather,
    Block,
    Hostile,
    Neutral,
    Player,
    Ambient,
    Voice,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum GameStateChangeReason {
    NoRespawnBlockAvailable,
    EndRaining,
    BeginRaining,
    ChangeGameMode,
    WinGame,
    DemoEvent,
    ArrowHitPlayer,
    RainLevelChange,
    ThunderLevelChange,
    PlayPufferfishStingSound,
    PlayElderGuardianMobAppearance,
    EnableRespawnScreen,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct ChunkDataBlockEntity {
    pub packed_xz: i8,
    pub y: i16,
    // TODO: block entity kind?
    pub kind: VarInt,
    pub data: Compound,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum GameMode {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub struct DeathLocation<'a> {
    pub dimension_name: Ident<&'a str>,
    pub position: BlockPos,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct AttributeProperty<'a> {
    pub key: Ident<&'a str>,
    pub value: f64,
    pub modifiers: Vec<AttributeModifier>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct AttributeModifier {
    pub uuid: Uuid,
    pub amount: f64,
    pub operation: u8,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct PlayerInfoAddPlayer<'a> {
    pub uuid: Uuid,
    pub username: Username<&'a str>,
    pub properties: Vec<SignedProperty<'a>>,
    pub game_mode: GameMode,
    pub ping: VarInt,
    pub display_name: Option<Text>,
    pub sig_data: Option<PublicKeyData<'a>>,
}
