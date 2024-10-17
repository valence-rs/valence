use crate::{Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct EntityTagQueryC2s {
    pub transaction_id: VarInt,
    pub entity_id: VarInt,
}
