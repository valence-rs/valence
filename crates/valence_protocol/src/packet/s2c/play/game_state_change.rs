use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x1c]
pub struct GameStateChangeS2c {
    pub kind: GameEventKind,
    pub value: f32,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum GameEventKind {
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
