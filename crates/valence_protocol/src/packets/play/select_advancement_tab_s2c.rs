use std::borrow::Cow;

use valence_ident::Ident;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SelectAdvancementTabS2c<'a> {
    pub identifier: Option<Ident<Cow<'a, str>>>,
}
