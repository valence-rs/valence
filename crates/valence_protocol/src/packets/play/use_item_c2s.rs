use crate::{Decode, Encode, Hand, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct UseItemC2s {
    pub hand: Hand,
    pub sequence: VarInt,
    pub yaw: f32,
    pub pitch: f32,
}
