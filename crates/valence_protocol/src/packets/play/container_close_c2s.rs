use crate::{Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct ContainerCloseC2s {
    pub window_id: VarInt,
}
