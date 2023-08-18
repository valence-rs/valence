use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct SpectatorTeleportC2s {
    pub target: Uuid,
}
