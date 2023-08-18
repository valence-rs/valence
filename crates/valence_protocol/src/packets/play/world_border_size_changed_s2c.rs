use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct WorldBorderSizeChangedS2c {
    pub diameter: f64,
}
