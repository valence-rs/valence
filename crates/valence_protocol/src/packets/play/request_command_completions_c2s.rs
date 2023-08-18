use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct RequestCommandCompletionsC2s<'a> {
    pub transaction_id: VarInt,
    pub text: Bounded<&'a str, 32500>,
}
