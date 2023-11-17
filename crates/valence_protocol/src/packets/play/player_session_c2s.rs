use std::borrow::Cow;

use uuid::Uuid;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct PlayerSessionC2s<'a>(pub Cow<'a, PlayerSessionData>);

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct PlayerSessionData {
    pub session_id: Uuid,
    // Public key
    pub expires_at: i64,
    pub public_key_data: Box<[u8]>,
    pub key_signature: Box<[u8]>,
}

impl<'a> From<PlayerSessionData> for Cow<'a, PlayerSessionData> {
    fn from(value: PlayerSessionData) -> Self {
        Cow::Owned(value)
    }
}

impl<'a> From<&'a PlayerSessionData> for Cow<'a, PlayerSessionData> {
    fn from(value: &'a PlayerSessionData) -> Self {
        Cow::Borrowed(value)
    }
}
