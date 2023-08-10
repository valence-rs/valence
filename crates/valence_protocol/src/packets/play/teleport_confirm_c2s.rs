use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::TELEPORT_CONFIRM_C2S)]
pub struct TeleportConfirmC2s {
    pub teleport_id: VarInt,
}
