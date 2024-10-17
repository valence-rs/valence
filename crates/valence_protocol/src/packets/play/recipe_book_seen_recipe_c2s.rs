use std::borrow::Cow;

use valence_ident::Ident;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct RecipeBookSeenRecipeC2s<'a> {
    pub recipe_id: Ident<Cow<'a, str>>,
}
