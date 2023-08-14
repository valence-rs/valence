use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CLIENT_COMMAND_C2S)]
pub struct ClientCommandC2s {
    pub entity_id: VarInt,
    pub action: ClientCommand,
    pub jump_boost: VarInt,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum ClientCommand {
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
