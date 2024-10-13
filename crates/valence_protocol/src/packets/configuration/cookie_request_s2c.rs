use std::borrow::Cow;
use valence_ident::Ident;
use crate::{Decode, Encode, Packet, PacketState, Text};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
/// Request the client to send the cookie with the specified key.
pub struct CookieRequestS2c<'a> {
    pub key: Ident<Cow<'a, str>>,
}
