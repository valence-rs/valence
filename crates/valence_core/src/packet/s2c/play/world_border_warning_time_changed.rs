use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct WorldBorderWarningTimeChangedS2c {
    pub warning_time: VarInt,
}
