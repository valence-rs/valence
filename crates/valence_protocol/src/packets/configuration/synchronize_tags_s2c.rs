use std::borrow::Cow;
use std::collections::BTreeMap;

use valence_ident::Ident;

use crate::{Decode, Encode, Packet, PacketState};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
pub struct SynchronizeTagsS2c {
    pub groups: Cow<'a, RegistryMap>,
}

pub type RegistryMap = BTreeMap<Ident<String>, BTreeMap<Ident<String>, Vec<VarInt>>>;