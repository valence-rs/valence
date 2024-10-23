use std::borrow::Cow;
use std::io::Write;

use anyhow::bail;
use valence_ident::Ident;

use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, PartialEq, Eq, Debug, Packet)]
pub struct RecipeS2c<'a> {
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

impl<'a> Decode<'a> for RecipeS2c<'a> {
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

impl Encode for RecipeS2c<'_> {
    fn encode(&self, _w: impl Write) -> anyhow::Result<()> {
        todo!()
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
