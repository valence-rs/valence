use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct LoginCompressionS2c {
    pub threshold: VarInt,
}
