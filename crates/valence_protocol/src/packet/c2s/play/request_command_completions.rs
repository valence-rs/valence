use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x08]
pub struct RequestCommandCompletionsC2s<'a> {
    pub transaction_id: VarInt,
    pub text: &'a str,
}
