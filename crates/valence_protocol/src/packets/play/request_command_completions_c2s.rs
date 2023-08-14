use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::REQUEST_COMMAND_COMPLETIONS_C2S)]
pub struct RequestCommandCompletionsC2s<'a> {
    pub transaction_id: VarInt,
    pub text: Bounded<&'a str, 32500>,
}
