use std::borrow::Cow;

use valence_ident::Ident;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct PlaceGhostRecipeS2c<'a> {
    pub window_id: VarInt,
    pub recipe_display: VarInt,
}
