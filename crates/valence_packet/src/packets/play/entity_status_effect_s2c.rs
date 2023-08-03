use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_STATUS_EFFECT_S2C)]
pub struct EntityStatusEffectS2c {
    pub entity_id: VarInt,
    pub effect_id: VarInt,
    pub amplifier: u8,
    pub duration: VarInt,
    pub flags: Flags,
    pub factor_codec: Option<Compound>,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, Encode, Decode)]
pub struct Flags {
    pub is_ambient: bool,
    pub show_particles: bool,
    pub show_icon: bool,
    #[bits(5)]
    _pad: u8,
}
