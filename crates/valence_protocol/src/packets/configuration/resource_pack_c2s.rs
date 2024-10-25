use uuid::Uuid;

use crate::packets::play::resource_pack_c2s::ResourcePackStatus;
use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Packet)]
pub struct ResourcePackC2s {
    uuid: Uuid,
    result: ResourcePackStatus,
}
