use crate::types::Hand;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x2f]
pub struct HandSwingC2s {
    pub hand: Hand,
}
