use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct PlayerInteractItemC2s {
    pub hand: Hand,
    pub sequence: VarInt,
}
