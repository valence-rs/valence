use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SetBorderWarningDelayS2c {
    pub warning_time: VarInt,
}
