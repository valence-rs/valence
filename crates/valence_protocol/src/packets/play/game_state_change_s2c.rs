use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::GAME_STATE_CHANGE_S2C)]
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
