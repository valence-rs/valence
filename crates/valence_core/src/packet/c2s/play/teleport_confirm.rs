use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct TeleportConfirmC2s {
    pub teleport_id: VarInt,
}
