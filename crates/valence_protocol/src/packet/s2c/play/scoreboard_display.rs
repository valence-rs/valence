use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x4d]
pub struct ScoreboardDisplayS2c<'a> {
    pub position: u8,
    pub score_name: &'a str,
}
