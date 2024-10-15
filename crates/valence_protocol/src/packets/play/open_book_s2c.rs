use crate::{Decode, Encode, Hand, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct OpenBookS2c {
    pub hand: Hand,
}
