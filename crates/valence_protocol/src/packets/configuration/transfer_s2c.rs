use std::borrow::Cow;

use valence_ident::Ident;

use crate::{Decode, Encode, Packet, PacketState, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
pub struct TransferS2c<'a> {
    pub host: Ident<Cow<'a, str>>,
    pub port: VarInt,
}
