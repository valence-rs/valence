use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet, PartialEq, Eq)]
#[packet(id = packet_id::UPDATE_PLAYER_ABILITIES_C2S)]
pub enum UpdatePlayerAbilitiesC2s {
    #[packet(tag = 0b00)]
    StopFlying,
    #[packet(tag = 0b10)]
    StartFlying,
}
