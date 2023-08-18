use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct RecipeBookDataC2s<'a> {
    pub recipe_id: Ident<Cow<'a, str>>,
}
