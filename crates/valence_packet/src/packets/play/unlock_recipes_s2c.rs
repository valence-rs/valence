use super::*;

#[derive(Clone, PartialEq, Eq, Debug, Packet)]
#[packet(id = packet_id::UNLOCK_RECIPES_S2C)]
pub struct UnlockRecipesS2c<'a> {
    pub action: UpdateRecipeBookAction<'a>,
    pub crafting_recipe_book_open: bool,
    pub crafting_recipe_book_filter_active: bool,
    pub smelting_recipe_book_open: bool,
    pub smelting_recipe_book_filter_active: bool,
    pub blast_furnace_recipe_book_open: bool,
    pub blast_furnace_recipe_book_filter_active: bool,
    pub smoker_recipe_book_open: bool,
    pub smoker_recipe_book_filter_active: bool,
    pub recipe_ids: Vec<Ident<Cow<'a, str>>>,
}

impl<'a> Decode<'a> for UnlockRecipesS2c<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let action_id = VarInt::decode(r)?.0;

        let crafting_recipe_book_open = bool::decode(r)?;
        let crafting_recipe_book_filter_active = bool::decode(r)?;
        let smelting_recipe_book_open = bool::decode(r)?;
        let smelting_recipe_book_filter_active = bool::decode(r)?;
        let blast_furnace_recipe_book_open = bool::decode(r)?;
        let blast_furnace_recipe_book_filter_active = bool::decode(r)?;
        let smoker_recipe_book_open = bool::decode(r)?;
        let smoker_recipe_book_filter_active = bool::decode(r)?;
        let recipe_ids = Vec::decode(r)?;

        Ok(Self {
            action: match action_id {
                0 => UpdateRecipeBookAction::Init {
                    recipe_ids: Vec::decode(r)?,
                },
                1 => UpdateRecipeBookAction::Add,
                2 => UpdateRecipeBookAction::Remove,
                n => bail!("unknown recipe book action of {n}"),
            },
            crafting_recipe_book_open,
            crafting_recipe_book_filter_active,
            smelting_recipe_book_open,
            smelting_recipe_book_filter_active,
            blast_furnace_recipe_book_open,
            blast_furnace_recipe_book_filter_active,
            smoker_recipe_book_open,
            smoker_recipe_book_filter_active,
            recipe_ids,
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum UpdateRecipeBookAction<'a> {
    Init {
        recipe_ids: Vec<Ident<Cow<'a, str>>>,
    },
    Add,
    Remove,
}
