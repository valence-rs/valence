use std::borrow::Cow;

use valence_protocol_macros::{Decode, Encode, Packet};

use crate::text_component::NbtText;
use crate::PacketState;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
/// Sent by the server to the client to disconnect them. The reason is displayed
/// to the client.
pub struct DisconnectS2c<'a> {
    pub reason: Cow<'a, NbtText>,
}

