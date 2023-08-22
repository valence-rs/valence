use std::borrow::Cow;
use std::collections::BTreeMap;

use valence_ident::Ident;

use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SynchronizeTagsS2c<'a> {
    pub groups: Cow<'a, RegistryMap>,
}

pub type RegistryMap = BTreeMap<Ident<String>, BTreeMap<Ident<String>, Vec<VarInt>>>;
