use crate::{BlockPos, Bounded, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct UpdateSignC2s<'a> {
    pub position: BlockPos,
    pub is_front_text: bool,
    pub lines: [Bounded<&'a str, 384>; 4],
}
