use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UPDATE_SIGN_C2S)]
pub struct UpdateSignC2s<'a> {
    pub position: BlockPos,
    pub is_front_text: bool,
    pub lines: [Bounded<&'a str, 384>; 4],
}
