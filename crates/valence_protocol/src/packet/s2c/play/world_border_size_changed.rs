use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x45]
pub struct WorldBorderSizeChangedS2c {
    pub diameter: f64,
}
