use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct RecipeBookAddS2c {
    pub recipe_count: VarInt,
    pub recipes: Vec<Recipe>,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct Recipe {
    pub recipe_id: VarInt,
    pub display_id: VarInt,
    pub group_id: Option<VarInt>,
    pub category_id: VarInt,
    pub has_ingredients: bool,
    pub ingredients: Option<Vec<VarInt>>,
    pub flags: u8,
    pub replace: bool,
}
