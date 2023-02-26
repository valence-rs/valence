use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct RequestCommandCompletionsC2s<'a> {
    pub transaction_id: VarInt,
    pub text: &'a str,
}
