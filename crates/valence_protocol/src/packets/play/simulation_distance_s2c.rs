use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SIMULATION_DISTANCE_S2C)]
pub struct SimulationDistanceS2c {
    pub simulation_distance: VarInt,
}
