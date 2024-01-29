use valence_nbt::Compound;

use crate::{Decode, Encode, Packet, PacketState};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
pub struct DynamicRegistriesS2c {
    pub codec: Compound
}
