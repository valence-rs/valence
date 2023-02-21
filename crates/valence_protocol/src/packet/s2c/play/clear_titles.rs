use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x0c]
pub struct ClearTitlesS2c {
    pub reset: bool,
}
