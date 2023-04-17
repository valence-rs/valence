use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub enum UpdatePlayerAbilitiesC2s {
    #[tag = 0b00]
    StopFlying,
    #[tag = 0b10]
    StartFlying,
}
