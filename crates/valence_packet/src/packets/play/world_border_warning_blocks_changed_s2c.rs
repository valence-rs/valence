use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::WORLD_BORDER_WARNING_BLOCKS_CHANGED_S2C)]
pub struct WorldBorderWarningBlocksChangedS2c {
    pub warning_blocks: VarInt,
}
