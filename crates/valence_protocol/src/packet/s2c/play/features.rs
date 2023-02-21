use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x67]
pub struct FeaturesS2c<'a> {
    pub features: Vec<Ident<&'a str>>,
}
