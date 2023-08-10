use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_PASSENGERS_SET_S2C)]
pub struct EntityPassengersSetS2c {
    /// Vehicle's entity id
    pub entity_id: VarInt,
    pub passengers: Vec<VarInt>,
}
