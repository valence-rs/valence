use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ROTATE_AND_MOVE_RELATIVE)]
pub struct RotateAndMoveRelativeS2c {
    pub entity_id: VarInt,
    pub delta: [i16; 3],
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
    pub on_ground: bool,
}
