use uuid::Uuid;

use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x30]
pub struct SpectatorTeleportC2s {
    pub target: Uuid,
}
