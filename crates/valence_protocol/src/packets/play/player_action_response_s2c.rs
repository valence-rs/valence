use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct PlayerActionResponseS2c {
    pub sequence: VarInt,
}
