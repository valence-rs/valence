use crate::{Decode, Encode, Hand, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct HandSwingC2s {
    pub hand: Hand,
}
