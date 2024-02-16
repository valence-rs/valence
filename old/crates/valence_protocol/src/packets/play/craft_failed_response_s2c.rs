use std::borrow::Cow;

use valence_ident::Ident;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct CraftFailedResponseS2c<'a> {
    pub window_id: u8,
    pub recipe: Ident<Cow<'a, str>>,
}
