use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct WorldBorderSizeChangedS2c {
    pub diameter: f64,
}
