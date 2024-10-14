use std::borrow::Cow;
use std::collections::BTreeSet;

use valence_ident::Ident;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct UpdateEnabledFeaturesS2c<'a> {
    pub features: Cow<'a, BTreeSet<Ident<String>>>,
}
