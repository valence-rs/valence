use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::HAND_SWING_C2S)]
pub struct HandSwingC2s {
    pub hand: Hand,
}
