use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct SimulationDistanceS2c {
    pub simulation_distance: VarInt,
}
