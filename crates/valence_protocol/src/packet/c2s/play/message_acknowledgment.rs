use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct MessageAcknowledgmentC2s {
    pub message_count: VarInt,
}
