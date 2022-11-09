use uuid::Uuid;

use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub enum HandshakeNextState {
    #[tag = 1]
    Status,
    #[tag = 2]
    Login,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub struct PublicKeyData<'a> {
    pub timestamp: u64,
    pub public_key: &'a [u8],
    pub signature: &'a [u8],
}

#[derive(Clone, Debug, Encode, Decode)]
pub enum MsgSigOrVerifyToken<'a> {
    MsgSig { salt: u64, sig: &'a [u8] },
    VerifyToken(&'a [u8]),
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct MessageAcknowledgment<'a> {
    pub last_seen: Vec<MessageAcknowledgmentEntry<'a>>,
    pub last_received: Option<MessageAcknowledgmentEntry<'a>>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct MessageAcknowledgmentEntry<'a> {
    pub profile_id: Uuid,
    pub signature: &'a [u8],
}

#[derive(Clone, Debug, Encode, Decode)]
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
