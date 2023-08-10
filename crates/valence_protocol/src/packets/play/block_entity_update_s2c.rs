use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::BLOCK_ENTITY_UPDATE_S2C)]
pub struct BlockEntityUpdateS2c<'a> {
    pub position: BlockPos,
    pub kind: VarInt,
    pub data: Cow<'a, Compound>,
}
