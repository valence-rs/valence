use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x1b]
pub enum UpdatePlayerAbilitiesC2s {
    #[tag = 0b00]
    StopFlying,
    #[tag = 0b10]
    StartFlying,
}
