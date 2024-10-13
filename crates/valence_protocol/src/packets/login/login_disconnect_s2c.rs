use std::borrow::Cow;

use crate::{Decode, Encode, Packet, PacketState, Text};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Login)]
/// Sent by the server to the client to disconnect the client from the server.
pub struct LoginDisconnectS2c<'a> {
    pub reason: Cow<'a, Text>,
}
