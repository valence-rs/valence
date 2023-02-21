use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x43]
pub struct WorldBorderCenterChangedS2c {
    pub xz_position: [f64; 2],
}
