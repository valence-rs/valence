use std::borrow::Cow;
use std::collections::BTreeSet;

use valence_ident::Ident;

use crate::{Decode, Encode, Packet, PacketState};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state= PacketState::Configuration)]
pub struct FeaturesS2c<'a> {
    pub features: Cow<'a, BTreeSet<Ident<String>>>,
}
