use std::borrow::Cow;

use valence_ident::Ident;

use crate::{Decode, Encode, Packet, PacketState};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
/// Stores a cookie on the client
pub struct StoreCookieS2c<'a> {
    pub key: Ident<Cow<'a, str>>,
    pub payload: Cow<'a, [u8]>,
}
