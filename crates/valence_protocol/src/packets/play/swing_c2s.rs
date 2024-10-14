use crate::{Decode, Encode, Hand, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct SwingC2s {
    pub hand: Hand,
}
