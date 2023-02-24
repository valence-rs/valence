use crate::ident::Ident;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct RecipeBookDataC2s<'a> {
    pub recipe_id: Ident<&'a str>,
}
