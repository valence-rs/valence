use std::borrow::Cow;

use valence_ident::Ident;

use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct RecipeBookSeenRecipeC2s {
    pub recipe_id: VarInt,
}
