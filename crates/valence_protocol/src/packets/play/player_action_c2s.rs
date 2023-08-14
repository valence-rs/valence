use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_ACTION_C2S)]
pub struct PlayerActionC2s {
    pub action: PlayerAction,
    pub position: BlockPos,
    pub direction: Direction,
    pub sequence: VarInt,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum PlayerAction {
    StartDestroyBlock,
    AbortDestroyBlock,
    StopDestroyBlock,
    DropAllItems,
    DropItem,
    ReleaseUseItem,
    SwapItemWithOffhand,
}
