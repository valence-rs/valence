use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::COOLDOWN_UPDATE_S2C)]
pub struct CooldownUpdateS2c {
    pub item_id: ItemKind,
    pub cooldown_ticks: VarInt,
}
