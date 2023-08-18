use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct OverlayMessageS2c<'a> {
    pub action_bar_text: Cow<'a, Text>,
}
