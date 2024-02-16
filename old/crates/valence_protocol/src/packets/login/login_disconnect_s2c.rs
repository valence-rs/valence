use std::borrow::Cow;

use crate::{Decode, Encode, Packet, PacketState, Text};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Login)]
pub struct LoginDisconnectS2c<'a> {
    pub reason: Cow<'a, Text>,
}
