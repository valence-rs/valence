use crate::var_long::VarLong;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct WorldBorderInterpolateSizeS2c {
    pub old_diameter: f64,
    pub new_diameter: f64,
    pub speed: VarLong,
}
