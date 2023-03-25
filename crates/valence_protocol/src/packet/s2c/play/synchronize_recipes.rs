use std::borrow::Cow;
use std::io::Write;

use anyhow::{bail, ensure};

use crate::ident::Ident;
use crate::item::ItemStack;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct SynchronizeRecipesS2c<'a> {
    pub recipes: Vec<Recipe<'a>>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Recipe<'a> {
    CraftingShapeless {
        recipe_id: Ident<Cow<'a, str>>,
        group: &'a str,
        category: CraftingCategory,
        ingredients: Vec<Ingredient>,
        result: Option<ItemStack>,
    },
    CraftingShaped {
        recipe_id: Ident<Cow<'a, str>>,
        width: VarInt,
        height: VarInt,
        group: &'a str,
        category: CraftingCategory,
        ingredients: Vec<Ingredient>,
        result: Option<ItemStack>,
    },
    CraftingSpecial {
        kind: SpecialCraftingKind,
        recipe_id: Ident<Cow<'a, str>>,
        category: CraftingCategory,
    },
    Smelting {
        recipe_id: Ident<Cow<'a, str>>,
        group: &'a str,
        category: SmeltCategory,
        ingredient: Ingredient,
        result: Option<ItemStack>,
        experience: f32,
        cooking_time: VarInt,
    },
    Blasting {
        recipe_id: Ident<Cow<'a, str>>,
        group: &'a str,
        category: SmeltCategory,
        ingredient: Ingredient,
        result: Option<ItemStack>,
        experience: f32,
        cooking_time: VarInt,
    },
    Smoking {
        recipe_id: Ident<Cow<'a, str>>,
        group: &'a str,
        category: SmeltCategory,
        ingredient: Ingredient,
        result: Option<ItemStack>,
        experience: f32,
        cooking_time: VarInt,
    },
    CampfireCooking {
        recipe_id: Ident<Cow<'a, str>>,
        group: &'a str,
        category: SmeltCategory,
        ingredient: Ingredient,
        result: Option<ItemStack>,
        experience: f32,
        cooking_time: VarInt,
    },
    Stonecutting {
        recipe_id: Ident<Cow<'a, str>>,
        group: &'a str,
        ingredient: Ingredient,
        result: Option<ItemStack>,
    },
    Smithing {
        recipe_id: Ident<Cow<'a, str>>,
        base: Ingredient,
        addition: Ingredient,
        result: Option<ItemStack>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SpecialCraftingKind {
    ArmorDye,
    BookCloning,
    MapCloning,
    MapExtending,
    FireworkRocket,
    FireworkStar,
    FireworkStarFade,
    RepairItem,
    TippedArrow,
    BannerDuplicate,
    BannerAddPattern,
    ShieldDecoration,
    ShulkerBoxColoring,
    SuspiciousStew,
}

/// Any item in the Vec may be used for the recipe.
pub type Ingredient = Vec<Option<ItemStack>>;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum CraftingCategory {
    Building,
    Redstone,
    Equipment,
    Misc,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum SmeltCategory {
    Food,
    Blocks,
    Misc,
}

impl<'a> Encode for Recipe<'a> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        match self {
            Recipe::CraftingShapeless {
                recipe_id,
                group,
                category,
                ingredients,
                result,
            } => {
                "crafting_shapeless".encode(&mut w)?;
                recipe_id.encode(&mut w)?;
                group.encode(&mut w)?;
                category.encode(&mut w)?;
                ingredients.encode(&mut w)?;
                result.encode(w)
            }
            Recipe::CraftingShaped {
                recipe_id,
                width,
                height,
                group,
                category,
                ingredients,
                result,
            } => {
                "crafting_shaped".encode(&mut w)?;
                recipe_id.encode(&mut w)?;
                width.encode(&mut w)?;
                height.encode(&mut w)?;
                group.encode(&mut w)?;
                category.encode(&mut w)?;

                ensure!(
                    (width.0 as usize).saturating_mul(height.0 as usize) == ingredients.len(),
                    "width * height must be equal to the number of ingredients"
                );

                for ing in ingredients {
                    ing.encode(&mut w)?;
                }

                result.encode(w)
            }
            Recipe::CraftingSpecial {
                kind,
                recipe_id,
                category,
            } => {
                match kind {
                    SpecialCraftingKind::ArmorDye => "crafting_special_armordye",
                    SpecialCraftingKind::BookCloning => "crafting_special_bookcloning",
                    SpecialCraftingKind::MapCloning => "crafting_special_mapcloning",
                    SpecialCraftingKind::MapExtending => "crafting_special_mapextending",
                    SpecialCraftingKind::FireworkRocket => "crafting_special_firework_rocket",
                    SpecialCraftingKind::FireworkStar => "crafting_special_firework_star",
                    SpecialCraftingKind::FireworkStarFade => "crafting_special_firework_star_fade",
                    SpecialCraftingKind::RepairItem => "crafting_special_repairitem",
                    SpecialCraftingKind::TippedArrow => "crafting_special_tippedarrow",
                    SpecialCraftingKind::BannerDuplicate => "crafting_special_bannerduplicate",
                    SpecialCraftingKind::BannerAddPattern => "crafting_special_banneraddpattern",
                    SpecialCraftingKind::ShieldDecoration => "crafting_special_shielddecoration",
                    SpecialCraftingKind::ShulkerBoxColoring => {
                        "crafting_special_shulkerboxcoloring"
                    }
                    SpecialCraftingKind::SuspiciousStew => "crafting_special_suspiciousstew",
                }
                .encode(&mut w)?;
                recipe_id.encode(&mut w)?;
                category.encode(w)
            }
            Recipe::Smelting {
                recipe_id,
                group,
                category,
                ingredient,
                result,
                experience,
                cooking_time,
            } => {
                "smelting".encode(&mut w)?;
                recipe_id.encode(&mut w)?;
                group.encode(&mut w)?;
                category.encode(&mut w)?;
                ingredient.encode(&mut w)?;
                result.encode(&mut w)?;
                experience.encode(&mut w)?;
                cooking_time.encode(w)
            }
            Recipe::Blasting {
                recipe_id,
                group,
                category,
                ingredient,
                result,
                experience,
                cooking_time,
            } => {
                "blasting".encode(&mut w)?;
                recipe_id.encode(&mut w)?;
                group.encode(&mut w)?;
                category.encode(&mut w)?;
                ingredient.encode(&mut w)?;
                result.encode(&mut w)?;
                experience.encode(&mut w)?;
                cooking_time.encode(w)
            }
            Recipe::Smoking {
                recipe_id,
                group,
                category,
                ingredient,
                result,
                experience,
                cooking_time,
            } => {
                "smoking".encode(&mut w)?;
                recipe_id.encode(&mut w)?;
                group.encode(&mut w)?;
                category.encode(&mut w)?;
                ingredient.encode(&mut w)?;
                result.encode(&mut w)?;
                experience.encode(&mut w)?;
                cooking_time.encode(w)
            }
            Recipe::CampfireCooking {
                recipe_id,
                group,
                category,
                ingredient,
                result,
                experience,
                cooking_time,
            } => {
                "campfire_cooking".encode(&mut w)?;
                recipe_id.encode(&mut w)?;
                group.encode(&mut w)?;
                category.encode(&mut w)?;
                ingredient.encode(&mut w)?;
                result.encode(&mut w)?;
                experience.encode(&mut w)?;
                cooking_time.encode(w)
            }
            Recipe::Stonecutting {
                recipe_id,
                group,
                ingredient,
                result,
            } => {
                "stonecutting".encode(&mut w)?;
                recipe_id.encode(&mut w)?;
                group.encode(&mut w)?;
                ingredient.encode(&mut w)?;
                result.encode(w)
            }
            Recipe::Smithing {
                recipe_id,
                base,
                addition,
                result,
            } => {
                "smithing".encode(&mut w)?;
                recipe_id.encode(&mut w)?;
                base.encode(&mut w)?;
                addition.encode(&mut w)?;
                result.encode(w)
            }
        }
    }
}

impl<'a> Decode<'a> for Recipe<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        Ok(match Ident::<Cow<str>>::decode(r)?.as_str() {
            "minecraft:crafting_shapeless" => Self::CraftingShapeless {
                recipe_id: Decode::decode(r)?,
                group: Decode::decode(r)?,
                category: Decode::decode(r)?,
                ingredients: Decode::decode(r)?,
                result: Decode::decode(r)?,
            },
            "minecraft:crafting_shaped" => {
                let recipe_id = Ident::decode(r)?;
                let width = VarInt::decode(r)?.0;
                let height = VarInt::decode(r)?.0;
                let group = <&str>::decode(r)?;
                let category = CraftingCategory::decode(r)?;

                let mut ingredients = Vec::new();
                for _ in 0..width.saturating_mul(height) {
                    ingredients.push(Ingredient::decode(r)?);
                }

                Self::CraftingShaped {
                    recipe_id,
                    width: VarInt(width),
                    height: VarInt(height),
                    group,
                    category,
                    ingredients,
                    result: Decode::decode(r)?,
                }
            }
            "minecraft:smelting" => Self::Smelting {
                recipe_id: Decode::decode(r)?,
                group: Decode::decode(r)?,
                category: Decode::decode(r)?,
                ingredient: Decode::decode(r)?,
                result: Decode::decode(r)?,
                experience: Decode::decode(r)?,
                cooking_time: Decode::decode(r)?,
            },
            "minecraft:blasting" => Self::Blasting {
                recipe_id: Decode::decode(r)?,
                group: Decode::decode(r)?,
                category: Decode::decode(r)?,
                ingredient: Decode::decode(r)?,
                result: Decode::decode(r)?,
                experience: Decode::decode(r)?,
                cooking_time: Decode::decode(r)?,
            },
            "minecraft:smoking" => Self::Smoking {
                recipe_id: Decode::decode(r)?,
                group: Decode::decode(r)?,
                category: Decode::decode(r)?,
                ingredient: Decode::decode(r)?,
                result: Decode::decode(r)?,
                experience: Decode::decode(r)?,
                cooking_time: Decode::decode(r)?,
            },
            "minecraft:campfire_cooking" => Self::CampfireCooking {
                recipe_id: Decode::decode(r)?,
                group: Decode::decode(r)?,
                category: Decode::decode(r)?,
                ingredient: Decode::decode(r)?,
                result: Decode::decode(r)?,
                experience: Decode::decode(r)?,
                cooking_time: Decode::decode(r)?,
            },
            "minecraft:stonecutting" => Self::Stonecutting {
                recipe_id: Decode::decode(r)?,
                group: Decode::decode(r)?,
                ingredient: Decode::decode(r)?,
                result: Decode::decode(r)?,
            },
            "minecraft:smithing" => Self::Smithing {
                recipe_id: Decode::decode(r)?,
                base: Decode::decode(r)?,
                addition: Decode::decode(r)?,
                result: Decode::decode(r)?,
            },
            other => Self::CraftingSpecial {
                kind: match other {
                    "minecraft:crafting_special_armordye" => SpecialCraftingKind::ArmorDye,
                    "minecraft:crafting_special_bookcloning" => SpecialCraftingKind::BookCloning,
                    "minecraft:crafting_special_mapcloning" => SpecialCraftingKind::MapCloning,
                    "minecraft:crafting_special_mapextending" => SpecialCraftingKind::MapExtending,
                    "minecraft:crafting_special_firework_rocket" => SpecialCraftingKind::FireworkRocket,
                    "minecraft:crafting_special_firework_star" => SpecialCraftingKind::FireworkStar,
                    "minecraft:crafting_special_firework_star_fade" => SpecialCraftingKind::FireworkStarFade,
                    "minecraft:crafting_special_repairitem" => SpecialCraftingKind::RepairItem,
                    "minecraft:crafting_special_tippedarrow" => SpecialCraftingKind::TippedArrow,
                    "minecraft:crafting_special_bannerduplicate" => SpecialCraftingKind::BannerDuplicate,
                    "minecraft:crafting_special_banneraddpattern" => SpecialCraftingKind::BannerAddPattern,
                    "minecraft:crafting_special_shielddecoration" => SpecialCraftingKind::ShieldDecoration,
                    "minecraft:crafting_special_shulkerboxcoloring" => {
                        SpecialCraftingKind::ShulkerBoxColoring
                    }
                    "minecraft:crafting_special_suspiciousstew" => SpecialCraftingKind::SuspiciousStew,
                    _ => bail!("unknown recipe type \"{other}\""),
                },
                recipe_id: Decode::decode(r)?,
                category: CraftingCategory::decode(r)?,
            },
        })
    }
}
