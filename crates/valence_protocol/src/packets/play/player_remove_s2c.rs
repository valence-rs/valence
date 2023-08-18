use super::*;

#[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
pub struct PlayerRemoveS2c<'a> {
    pub uuids: Cow<'a, [Uuid]>,
}
