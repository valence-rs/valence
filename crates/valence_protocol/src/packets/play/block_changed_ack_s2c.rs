use crate::{Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct BlockChangedAckS2c {
    pub sequence: VarInt,
}
