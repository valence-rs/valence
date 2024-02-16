use std::borrow::Cow;

use valence_ident::Ident;

use crate::{Bounded, Decode, Encode, Packet, PacketState, RawBytes, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Login)]
pub struct LoginQueryRequestS2c<'a> {
    pub message_id: VarInt,
    pub channel: Ident<Cow<'a, str>>,
    pub data: Bounded<RawBytes<'a>, 1048576>,
}
