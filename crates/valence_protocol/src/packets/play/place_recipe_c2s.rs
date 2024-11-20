use std::borrow::Cow;

use valence_ident::Ident;

use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct PlaceRecipeC2s {
    pub window_id: i8,
    pub recipe: VarInt,
    pub make_all: bool,
}
