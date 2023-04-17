use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct WorldBorderCenterChangedS2c {
    pub x_pos: f64,
    pub z_pos: f64,
}
