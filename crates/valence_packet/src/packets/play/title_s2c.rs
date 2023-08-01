use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::TITLE_S2C)]
pub struct TitleS2c<'a> {
    pub title_text: Cow<'a, Text>,
}
