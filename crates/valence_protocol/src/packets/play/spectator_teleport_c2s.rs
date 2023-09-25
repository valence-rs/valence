use uuid::Uuid;

use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct SpectatorTeleportC2s {
    pub target: Uuid,
}
