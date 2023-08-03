use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_INTERACT_BLOCK_C2S)]
pub struct PlayerInteractBlockC2s {
    pub hand: Hand,
    pub position: BlockPos,
    pub face: Direction,
    pub cursor_pos: Vec3,
    pub head_inside_block: bool,
    pub sequence: VarInt,
}
