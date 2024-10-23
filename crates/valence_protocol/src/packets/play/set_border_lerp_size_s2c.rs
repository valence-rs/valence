use crate::{Decode, Encode, Packet, VarLong};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SetBorderLerpSizeS2c {
    pub old_diameter: f64,
    pub new_diameter: f64,
    pub duration_millis: VarLong,
}
