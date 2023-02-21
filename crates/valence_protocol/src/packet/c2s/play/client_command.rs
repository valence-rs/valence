use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x1d]
pub struct ClientCommandC2s {
    pub entity_id: VarInt,
    pub action: Action,
    pub jump_boost: VarInt,
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
