use std::borrow::Cow;
use std::io::Write;

use anyhow::bail;

use crate::ident::Ident;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, PartialEq, Eq, Debug)]
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

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum UpdateRecipeBookAction<'a> {
    Init { recipe_ids: Vec<Ident<Cow<'a, str>>> },
    Add,
    Remove,
}

impl Encode for UnlockRecipesS2c<'_> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        VarInt(match &self.action {
            UpdateRecipeBookAction::Init { .. } => 0,
            UpdateRecipeBookAction::Add => 1,
            UpdateRecipeBookAction::Remove => 2,
        })
        .encode(&mut w)?;

        self.crafting_recipe_book_open.encode(&mut w)?;
        self.crafting_recipe_book_filter_active.encode(&mut w)?;
        self.smelting_recipe_book_open.encode(&mut w)?;
        self.smelting_recipe_book_filter_active.encode(&mut w)?;
        self.blast_furnace_recipe_book_open.encode(&mut w)?;
        self.blast_furnace_recipe_book_filter_active
            .encode(&mut w)?;
        self.smoker_recipe_book_open.encode(&mut w)?;
        self.smoker_recipe_book_filter_active.encode(&mut w)?;
        self.recipe_ids.encode(&mut w)?;
        if let UpdateRecipeBookAction::Init { recipe_ids } = &self.action {
            recipe_ids.encode(&mut w)?;
        }

        Ok(())
    }
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
