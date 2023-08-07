use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::RECIPE_BOOK_DATA_C2S)]
pub struct RecipeBookDataC2s<'a> {
    pub recipe_id: Ident<Cow<'a, str>>,
}
