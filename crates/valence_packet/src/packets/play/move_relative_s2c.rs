use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::MOVE_RELATIVE)]
pub struct MoveRelativeS2c {
    pub entity_id: VarInt,
    pub delta: [i16; 3],
    pub on_ground: bool,
}
