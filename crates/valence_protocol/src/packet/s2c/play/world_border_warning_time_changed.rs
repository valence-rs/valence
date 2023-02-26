use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct WorldBorderWarningTimeChangedS2c {
    pub warning_time: VarInt,
}
