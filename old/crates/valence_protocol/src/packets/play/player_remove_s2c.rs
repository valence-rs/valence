use std::borrow::Cow;

use uuid::Uuid;

use crate::{Decode, Encode, Packet};

#[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
pub struct PlayerRemoveS2c<'a> {
    pub uuids: Cow<'a, [Uuid]>,
}
