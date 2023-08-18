use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SimulationDistanceS2c {
    pub simulation_distance: VarInt,
}
