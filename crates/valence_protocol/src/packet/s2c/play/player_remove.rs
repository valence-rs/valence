use std::borrow::Cow;

use uuid::Uuid;

use crate::{Decode, Encode};

#[derive(Clone, PartialEq, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x35]
pub struct PlayerRemoveS2c<'a> {
    pub uuids: Cow<'a, [Uuid]>,
}
