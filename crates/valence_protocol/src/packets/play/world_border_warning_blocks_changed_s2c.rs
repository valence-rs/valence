use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct WorldBorderWarningBlocksChangedS2c {
    pub warning_blocks: VarInt,
}
