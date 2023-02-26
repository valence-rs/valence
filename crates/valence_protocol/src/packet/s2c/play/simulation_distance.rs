use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct SimulationDistanceS2c {
    pub simulation_distance: VarInt,
}
