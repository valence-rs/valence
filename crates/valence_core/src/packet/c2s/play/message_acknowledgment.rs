use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct MessageAcknowledgmentC2s {
    pub message_index: VarInt,
}
