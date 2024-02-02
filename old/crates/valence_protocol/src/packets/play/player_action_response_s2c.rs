use crate::{Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct PlayerActionResponseS2c {
    pub sequence: VarInt,
}
