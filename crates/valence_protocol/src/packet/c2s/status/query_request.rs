use crate::{Decode, DecodePacket, Encode, EncodePacket};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x00]
pub struct QueryRequestC2s;
