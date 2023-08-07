use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::STATISTICS_S2C)]
pub struct StatisticsS2c {
    pub statistics: Vec<Statistic>,
}

#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode)]
pub struct Statistic {
    pub category_id: VarInt,
    pub statistic_id: VarInt,
    pub value: VarInt,
}
