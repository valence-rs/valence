use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct SetCarriedItemS2c {
    pub slot: u8,
}
