use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct ContainerCloseC2s {
    pub window_id: i8,
}
