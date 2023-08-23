use crate::{Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct TeleportConfirmC2s {
    pub teleport_id: VarInt,
}
