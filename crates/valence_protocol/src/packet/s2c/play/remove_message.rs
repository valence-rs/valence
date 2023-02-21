use std::io::Write;

use crate::types::MessageSignature;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x16]
pub struct RemoveMessageS2c<'a> {
    pub signature: MessageSignature<'a>,
}
