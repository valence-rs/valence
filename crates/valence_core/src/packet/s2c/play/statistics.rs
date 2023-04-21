use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct StatisticsS2c {
    pub statistics: Vec<Statistic>,
}

#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode)]
pub struct Statistic {
    pub category_id: VarInt,
    pub statistic_id: VarInt,
    pub value: VarInt,
}
