use std::borrow::Cow;

use valence_ident::Ident;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct PlaceRecipeC2s<'a> {
    pub window_id: i8,
    pub recipe: Ident<Cow<'a, str>>,
    pub make_all: bool,
}
