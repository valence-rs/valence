use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SubtitleS2c<'a> {
    pub subtitle_text: Cow<'a, Text>,
}
