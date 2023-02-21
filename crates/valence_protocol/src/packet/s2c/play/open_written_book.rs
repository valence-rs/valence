use crate::types::Hand;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x2b]
pub struct OpenWrittenBookS2c {
    pub hand: Hand,
}
