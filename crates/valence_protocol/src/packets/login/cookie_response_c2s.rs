use std::borrow::Cow;
use valence_ident::Ident;
use crate::{Decode, Encode, Packet, PacketState, Text};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Login)]
/// Sent by the client to the server to respond to a 
/// [CookieRequestS2c](crate::packets::login::CookieRequestS2c) packet.
pub struct CookieResponseC2s<'a> {
    pub key: Ident<Cow<'a, str>>,
    pub has_payload: bool,
    pub payload: Option<Cow<'a, [u8]>>,
}
