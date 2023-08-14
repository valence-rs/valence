use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::RECIPE_CATEGORY_OPTIONS_C2S)]
pub struct RecipeCategoryOptionsC2s {
    pub book_id: RecipeBookId,
    pub book_open: bool,
    pub filter_active: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum RecipeBookId {
    Crafting,
    Furnace,
    BlastFurnace,
    Smoker,
}
