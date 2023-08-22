use std::borrow::Cow;

use valence_ident::Ident;

use crate::{Bounded, Decode, Encode, Packet, RawBytes};

pub const MAX_PAYLOAD_SIZE: usize = 32767;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct CustomPayloadC2s<'a> {
    pub channel: Ident<Cow<'a, str>>,
    pub data: Bounded<RawBytes<'a>, MAX_PAYLOAD_SIZE>,
}
