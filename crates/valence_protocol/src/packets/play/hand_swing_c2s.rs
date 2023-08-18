use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct HandSwingC2s {
    pub hand: Hand,
}
