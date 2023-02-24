use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ExperienceBarUpdateS2c {
    pub bar: f32,
    pub level: VarInt,
    pub total_xp: VarInt,
}
