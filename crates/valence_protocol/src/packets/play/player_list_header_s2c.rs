use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct PlayerListHeaderS2c<'a> {
    pub header: Cow<'a, Text>,
    pub footer: Cow<'a, Text>,
}
