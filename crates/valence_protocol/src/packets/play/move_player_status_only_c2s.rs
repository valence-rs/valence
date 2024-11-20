use crate::{movement_flags::MovementFlags, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct MovePlayerStatusOnlyC2s {
    pub flags: MovementFlags,
}
