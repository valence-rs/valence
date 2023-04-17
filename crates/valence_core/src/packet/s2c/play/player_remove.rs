use std::borrow::Cow;

use uuid::Uuid;

use crate::packet::{Decode, Encode};

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct PlayerRemoveS2c<'a> {
    pub uuids: Cow<'a, [Uuid]>,
}
