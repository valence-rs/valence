use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct WorldBorderInterpolateSizeS2c {
    pub old_diameter: f64,
    pub new_diameter: f64,
    pub duration_millis: VarLong,
}
