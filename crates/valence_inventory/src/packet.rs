//! Inventory packets

use std::borrow::Cow;
use std::io::Write;

use anyhow::bail;
use valence_core::ident::Ident;
use valence_core::item::ItemStack;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{packet_id, Decode, Encode, Packet};
use valence_core::text::Text;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CLICK_SLOT_C2S)]
pub struct ClickSlotC2s {
    pub window_id: u8,
    pub state_id: VarInt,
    pub slot_idx: i16,
    /// The button used to click the slot. An enum can't easily be used for this
    /// because the meaning of this value depends on the mode.
    pub button: i8,
    pub mode: ClickMode,
    pub slot_changes: Vec<SlotChange>,
    pub carried_item: Option<ItemStack>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub enum ClickMode {
    Click,
    ShiftClick,
    Hotbar,
    CreativeMiddleClick,
    DropKey,
    Drag,
    DoubleClick,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct SlotChange {
    pub idx: i16,
    pub item: Option<ItemStack>,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CLOSE_HANDLED_SCREEN_C2S)]
pub struct CloseHandledScreenC2s {
    pub window_id: i8,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CREATIVE_INVENTORY_ACTION_C2S)]
pub struct CreativeInventoryActionC2s {
    pub slot: i16,
    pub clicked_item: Option<ItemStack>,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UPDATE_SELECTED_SLOT_C2S)]
pub struct UpdateSelectedSlotC2s {
    pub slot: i16,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CLOSE_SCREEN_S2C)]
pub struct CloseScreenS2c {
    /// Ignored by notchian clients.
    pub window_id: u8,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::INVENTORY_S2C)]
pub struct InventoryS2c<'a> {
    pub window_id: u8,
    pub state_id: VarInt,
    pub slots: Cow<'a, [Option<ItemStack>]>,
    pub carried_item: Cow<'a, Option<ItemStack>>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum WindowType {
    Generic9x1,
    Generic9x2,
    Generic9x3,
    Generic9x4,
    Generic9x5,
    Generic9x6,
    Generic3x3,
    Anvil,
    Beacon,
    BlastFurnace,
    BrewingStand,
    Crafting,
    Enchantment,
    Furnace,
    Grindstone,
    Hopper,
    Lectern,
    Loom,
    Merchant,
    ShulkerBox,
    Smithing,
    Smoker,
    Cartography,
    Stonecutter,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::OPEN_SCREEN_S2C)]
pub struct OpenScreenS2c<'a> {
    pub window_id: VarInt,
    pub window_type: WindowType,
    pub window_title: Cow<'a, Text>,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::OPEN_HORSE_SCREEN_S2C)]
pub struct OpenHorseScreenS2c {
    pub window_id: u8,
    pub slot_count: VarInt,
    pub entity_id: i32,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SCREEN_HANDLER_SLOT_UPDATE_S2C)]
pub struct ScreenHandlerSlotUpdateS2c<'a> {
    pub window_id: i8,
    pub state_id: VarInt,
    pub slot_idx: i16,
    pub slot_data: Cow<'a, Option<ItemStack>>,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SCREEN_HANDLER_PROPERTY_UPDATE_S2C)]
pub struct ScreenHandlerPropertyUpdateS2c {
    pub window_id: u8,
    pub property: i16,
    pub value: i16,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CRAFT_REQUEST_C2S)]
pub struct CraftRequestC2s<'a> {
    pub window_id: i8,
    pub recipe: Ident<Cow<'a, str>>,
    pub make_all: bool,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CRAFT_FAILED_RESPONSE_S2C)]
pub struct CraftFailedResponseS2c<'a> {
    pub window_id: u8,
    pub recipe: Ident<Cow<'a, str>>,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PICK_FROM_INVENTORY_C2S)]
pub struct PickFromInventoryC2s {
    pub slot_to_use: VarInt,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SET_TRADE_OFFERS_S2C)]
pub struct SetTradeOffersS2c {
    pub window_id: VarInt,
    pub trades: Vec<TradeOffer>,
    pub villager_level: VarInt,
    pub experience: VarInt,
    pub is_regular_villager: bool,
    pub can_restock: bool,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct TradeOffer {
    pub input_one: Option<ItemStack>,
    pub output_item: Option<ItemStack>,
    pub input_two: Option<ItemStack>,
    pub trade_disabled: bool,
    pub number_of_trade_uses: i32,
    pub max_trade_uses: i32,
    pub xp: i32,
    pub special_price: i32,
    pub price_multiplier: f32,
    pub demand: i32,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::BUTTON_CLICK_C2S)]
pub struct ButtonClickC2s {
    pub window_id: i8,
    pub button_id: i8,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::RECIPE_BOOK_DATA_C2S)]
pub struct RecipeBookDataC2s<'a> {
    pub recipe_id: Ident<Cow<'a, str>>,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::RENAME_ITEM_C2S)]
pub struct RenameItemC2s<'a> {
    pub item_name: &'a str,
}

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

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SELECT_MERCHANT_TRADE_C2S)]
pub struct SelectMerchantTradeC2s {
    pub selected_slot: VarInt,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UPDATE_BEACON_C2S)]
pub struct UpdateBeaconC2s {
    pub primary_effect: Option<VarInt>,
    pub secondary_effect: Option<VarInt>,
}

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

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum UpdateRecipeBookAction<'a> {
    Init {
        recipe_ids: Vec<Ident<Cow<'a, str>>>,
    },
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

pub mod synchronize_recipes {
    use anyhow::ensure;

    use super::*;

    #[derive(Clone, Debug, Encode, Decode, Packet)]
    #[packet(id = packet_id::SYNCHRONIZE_RECIPES_S2C)]
    pub struct SynchronizeRecipesS2c<'a> {
        // TODO: this should be a Vec<Recipe<'a>>
        pub recipes: valence_core::protocol::raw::RawBytes<'a>,
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
                        SpecialCraftingKind::FireworkStarFade => {
                            "crafting_special_firework_star_fade"
                        }
                        SpecialCraftingKind::RepairItem => "crafting_special_repairitem",
                        SpecialCraftingKind::TippedArrow => "crafting_special_tippedarrow",
                        SpecialCraftingKind::BannerDuplicate => "crafting_special_bannerduplicate",
                        SpecialCraftingKind::BannerAddPattern => {
                            "crafting_special_banneraddpattern"
                        }
                        SpecialCraftingKind::ShieldDecoration => {
                            "crafting_special_shielddecoration"
                        }
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
                        "minecraft:crafting_special_bookcloning" => {
                            SpecialCraftingKind::BookCloning
                        }
                        "minecraft:crafting_special_mapcloning" => SpecialCraftingKind::MapCloning,
                        "minecraft:crafting_special_mapextending" => {
                            SpecialCraftingKind::MapExtending
                        }
                        "minecraft:crafting_special_firework_rocket" => {
                            SpecialCraftingKind::FireworkRocket
                        }
                        "minecraft:crafting_special_firework_star" => {
                            SpecialCraftingKind::FireworkStar
                        }
                        "minecraft:crafting_special_firework_star_fade" => {
                            SpecialCraftingKind::FireworkStarFade
                        }
                        "minecraft:crafting_special_repairitem" => SpecialCraftingKind::RepairItem,
                        "minecraft:crafting_special_tippedarrow" => {
                            SpecialCraftingKind::TippedArrow
                        }
                        "minecraft:crafting_special_bannerduplicate" => {
                            SpecialCraftingKind::BannerDuplicate
                        }
                        "minecraft:crafting_special_banneraddpattern" => {
                            SpecialCraftingKind::BannerAddPattern
                        }
                        "minecraft:crafting_special_shielddecoration" => {
                            SpecialCraftingKind::ShieldDecoration
                        }
                        "minecraft:crafting_special_shulkerboxcoloring" => {
                            SpecialCraftingKind::ShulkerBoxColoring
                        }
                        "minecraft:crafting_special_suspiciousstew" => {
                            SpecialCraftingKind::SuspiciousStew
                        }
                        _ => bail!("unknown recipe type \"{other}\""),
                    },
                    recipe_id: Decode::decode(r)?,
                    category: CraftingCategory::decode(r)?,
                },
            })
        }
    }
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::COOLDOWN_UPDATE_S2C)]
pub struct CooldownUpdateS2c {
    pub item_id: VarInt,
    pub cooldown_ticks: VarInt,
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UPDATE_SELECTED_SLOT_S2C)]
pub struct UpdateSelectedSlotS2c {
    pub slot: u8,
}
