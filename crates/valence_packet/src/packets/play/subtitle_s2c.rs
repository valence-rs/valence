use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SUBTITLE_S2C)]
pub struct SubtitleS2c<'a> {
    pub subtitle_text: Cow<'a, Text>,
}
