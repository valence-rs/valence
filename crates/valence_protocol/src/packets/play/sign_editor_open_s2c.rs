use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SIGN_EDITOR_OPEN_S2C)]
pub struct SignEditorOpenS2c {
    pub location: BlockPos,
}
