use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x58]
pub struct SimulationDistanceS2c {
    pub simulation_distance: VarInt,
}
