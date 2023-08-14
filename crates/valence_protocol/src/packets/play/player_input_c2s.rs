use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_INPUT_C2S)]
pub struct PlayerInputC2s {
    pub sideways: f32,
    pub forward: f32,
    pub flags: PlayerInputFlags,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct PlayerInputFlags {
    pub jump: bool,
    pub unmount: bool,
    #[bits(6)]
    _pad: u8,
}
