use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct WorldBorderCenterChangedS2c {
    pub xz_position: [f64; 2],
}
