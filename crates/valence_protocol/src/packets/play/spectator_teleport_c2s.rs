use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SPECTATOR_TELEPORT_C2S)]
pub struct SpectatorTeleportC2s {
    pub target: Uuid,
}
