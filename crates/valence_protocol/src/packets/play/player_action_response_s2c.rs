use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_ACTION_RESPONSE_S2C)]
pub struct PlayerActionResponseS2c {
    pub sequence: VarInt,
}
