use std::borrow::Cow;

use crate::ident::Ident;
use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct RecipeBookDataC2s<'a> {
    pub recipe_id: Ident<Cow<'a, str>>,
}
