use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct TitleS2c<'a> {
    pub title_text: Cow<'a, Text>,
}
