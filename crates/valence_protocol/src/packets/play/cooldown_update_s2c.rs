use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct CooldownUpdateS2c {
    pub item_id: ItemKind,
    pub cooldown_ticks: VarInt,
}
