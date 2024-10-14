use crate::{Decode, Encode, ItemKind, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct CooldownS2c {
    pub item_id: ItemKind,
    pub cooldown_ticks: VarInt,
}
