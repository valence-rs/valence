use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SetBorderSizeS2c {
    pub diameter: f64,
}
