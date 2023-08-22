use std::borrow::Cow;
use std::io::Write;

use anyhow::ensure;
use valence_ident::Ident;

use crate::{Decode, Encode, ItemStack, Packet, RawBytes};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SynchronizeRecipesS2c<'a> {
    // TODO: this should be a Vec<Recipe<'a>>
    pub recipes: RawBytes<'a>,
}

#[derive(Clone, Debug, Encode)]
pub struct Recipe<'a> {
    pub kind: Ident<Cow<'a, str>>,
    pub recipe_id: Ident<Cow<'a, str>>,
    pub data: RecipeData<'a>,
}

#[derive(Clone, Debug, Encode)]
pub enum RecipeData<'a> {
    CraftingShapeless(CraftingShapedData<'a>),
    // TODO: fill in the rest.
    CraftingShaped,
    CraftingSpecialArmordye,
    CraftingSpecialBookcloning,
    CraftingSpecialMapcloning,
    CraftingSpecialMapextending,
    CraftingSpecialFireworkRocket,
    CraftingSpecialFireworkStar,
    CraftingSpecialFireworkStarFade,
    CraftingSpecialRepairitem,
    CraftingSpecialTippedarrow,
    CraftingSpecialBannerduplicate,
    CraftingSpecialShielddecoration,
    CraftingSpecialShulkerboxcoloring,
    CraftingSpecialSuspiciousStew,
    CraftingDecoratedPot,
    Smelting,
    Blasting,
    Smoking,
    CampfireCooking,
    Stonecutting,
    SmithingTransform,
    SmithingTrim,
}

#[derive(Clone, Debug)]
pub struct CraftingShapedData<'a> {
    pub width: u32,
    pub height: u32,
    pub group: &'a str,
    pub category: CraftingShapedCategory,
    /// Length must be width * height.
    pub ingredients: Cow<'a, [Ingredient<'a>]>,
    pub result: Option<ItemStack>,
    pub show_notification: bool,
}

impl Encode for CraftingShapedData<'_> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        let Self {
            width,
            height,
            group,
            category,
            ingredients,
            result,
            show_notification,
        } = self;

        width.encode(&mut w)?;
        height.encode(&mut w)?;
        group.encode(&mut w)?;
        category.encode(&mut w)?;

        let len = width
            .checked_mul(*height)
            .expect("bad shaped recipe dimensions") as usize;

        ensure!(
            len == ingredients.len(),
            "number of ingredients in shaped recipe must be equal to width * height"
        );

        for ingr in ingredients.as_ref() {
            ingr.encode(&mut w)?;
        }

        result.encode(&mut w)?;

        show_notification.encode(w)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum CraftingShapedCategory {
    Building,
    Redstone,
    Equipment,
    Misc,
}

pub type Ingredient<'a> = Cow<'a, [Option<ItemStack>]>;
