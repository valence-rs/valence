use anyhow::Ok;
use heck::ToPascalCase;
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;

use crate::{
    block::TopLevel,
    ident,
    item_block_convert::{block_to_item_arms, item_to_block_arms},
};

#[derive(Deserialize, Clone, Debug)]
pub(crate) struct Item {
    #[allow(unused)]
    pub(crate) id: u16,
    translation_key: String,
    pub(crate) name: String,
    max_stack: u8,
    food: Option<FoodComponent>,
    max_damage: Option<i16>,
    enchantability: Option<u8>,
    fireproof: Option<bool>,
}

#[derive(Deserialize, Clone, Debug)]
struct FoodComponent {
    hunger: u8,
    saturation: f32,
    always_edible: bool,
    meat: bool,
    snack: bool,
    // TODO: Implement when postions are implemented.
    //effects: Vec<()>,
}

pub fn build() -> anyhow::Result<TokenStream> {
    let items = serde_json::from_str::<Vec<Item>>(include_str!("../extracted/items.json"))?;

    let item_kinds_count = items.len();

    let item_kind_to_translation_key_arms = items
        .iter()
        .map(|i| {
            let item = ident(i.name.to_pascal_case());
            let translation_key = &i.translation_key;
            quote! {
                Self::#item => #translation_key,
            }
        })
        .collect::<TokenStream>();

    let item_kind_variants = items
        .iter()
        .map(|i| ident(i.name.to_pascal_case()))
        .collect::<Vec<_>>();

    let item_kind_from_str_arms = items
        .iter()
        .map(|i| {
            let name = &i.name;
            let name_ident = ident(name.to_pascal_case());
            quote! {
                #name => Some(Self::#name_ident),
            }
        })
        .collect::<TokenStream>();

    let item_kind_to_str_arms = items
        .iter()
        .map(|i| {
            let name = &i.name;
            let name_ident = ident(name.to_pascal_case());
            quote! {
                Self::#name_ident => #name,
            }
        })
        .collect::<TokenStream>();

    let item_kind_to_raw_id_arms = items
        .iter()
        .map(|i| {
            let id = &i.id;
            let name_ident = ident(&i.name.to_pascal_case());

            quote! {
                Self::#name_ident => #id,
            }
        })
        .collect::<TokenStream>();

    let item_kind_from_raw_id_arms = items
        .iter()
        .map(|i| {
            let id = &i.id;
            let name_ident = ident(&i.name.to_pascal_case());

            quote! {
                #id => Some(Self::#name_ident),
            }
        })
        .collect::<TokenStream>();

    let item_kind_to_max_stack_arms = items
        .iter()
        .map(|i| {
            let name_ident = ident(&i.name.to_pascal_case());
            let max_count = i.max_stack;

            quote! {
                Self::#name_ident => #max_count,
            }
        })
        .collect::<TokenStream>();

    let item_kind_to_food_component_arms = items
        .iter()
        .map(|i| {
            match &i.food {
                Some(food_component) => {
                    let name_ident = ident(&i.name.to_pascal_case());
                    let hunger = food_component.hunger;
                    let saturation = food_component.saturation;
                    let always_edible = food_component.always_edible;
                    let meat = food_component.meat;
                    let snack = food_component.snack;
                    //let effects = food_component.effects;

                    quote! {
                        Self::#name_ident => Some(FoodComponent {
                            hunger: #hunger,
                            saturation: #saturation,
                            always_edible: #always_edible,
                            meat: #meat,
                            snack: #snack,
                            //effects: #effects,
                        }
                    ),
                    }
                }
                None => quote! {},
            }
        })
        .collect::<TokenStream>();

    let item_kind_to_max_durability_arms = items
        .iter()
        .map(|i| match &i.max_damage {
            Some(max_durability) => {
                let name_ident = ident(&i.name.to_pascal_case());

                quote! {
                    Self::#name_ident => Some(#max_durability),
                }
            }
            None => quote! {},
        })
        .collect::<TokenStream>();

    let item_kind_to_enchantability_arms = items
        .iter()
        .map(|i| match &i.enchantability {
            Some(enchantability) => {
                let name_ident = ident(&i.name.to_pascal_case());

                quote! {
                    Self::#name_ident => Some(#enchantability),
                }
            }
            None => quote! {},
        })
        .collect::<TokenStream>();

    let item_kind_to_fireproof_arms = items
        .iter()
        .map(|i| match i.fireproof {
            Some(_) => {
                let name_ident = ident(&i.name.to_pascal_case());

                quote! {
                    Self::#name_ident => true,
                }
            }
            None => quote! {},
        })
        .collect::<TokenStream>();

    let item_kinds_varients = items
        .iter()
        .map(|i| ident(i.name.to_pascal_case()))
        .collect::<Vec<_>>();

    let TopLevel { blocks, .. } = serde_json::from_str(include_str!("../extracted/blocks.json"))?;

    let block_kind_to_item_kind_arms = block_to_item_arms(&blocks, &items);

    let item_kind_to_block_kind_arms = item_to_block_arms(&blocks, &items);

    Ok(quote! {
        /// Represents an item from the game
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        #[repr(u16)]
        pub enum ItemKind {
            #(#item_kind_variants,)*
        }

        impl ItemKind {
            /// Constructs a item kind from a raw item ID.
            ///
            /// If the given ID is invalid, `None` is returned.
            pub const fn from_raw(id: u16) -> Option<Self> {
                match id {
                    #item_kind_from_raw_id_arms
                    _ => None
                }
            }

            /// Gets the raw item ID from the item kind
            pub const fn to_raw(self) -> u16 {
                match self {
                    #item_kind_to_raw_id_arms
                }
            }

            /// Construct an item kind for its snake_case name.
            ///
            /// Returns `None` if the name is invalid.
            // Added to make it the same as BlockKind
            #[allow(clippy::should_implement_trait)]
            pub fn from_str(name: &str) -> Option<ItemKind> {
                match name {
                    #item_kind_from_str_arms
                    _ => None
                }
            }

            /// Gets the snake_case name of this item kind.
            pub const fn to_str(self) -> &'static str {
                match self {
                    #item_kind_to_str_arms
                }
            }

            /// Gets the translation key of this item kind.
            pub const fn translation_key(self) -> &'static str {
                match self {
                    #item_kind_to_translation_key_arms
                }
            }

            /// Returns the max stack count
            pub const fn max_stack(self) -> u8 {
                match self {
                    #item_kind_to_max_stack_arms
                }
            }

            /// Returns a food component which stores hunger, saturation etc.
            ///
            /// If the item kind can't be eaten, `None` will be returned.
            pub const fn food_component(self) -> Option<FoodComponent> {
                match self {
                    #item_kind_to_food_component_arms
                    _ => None
                }
            }

            /// Returns the max durability before the item kind will break
            ///
            /// If the item kind doesn't have durability, `None` is returned.
            pub const fn max_durability(self) -> Option<i16> {
                match self {
                    #item_kind_to_max_durability_arms
                    _ => None
                }
            }

            /// Returns the enchantability of the item kind
            ///
            /// If the item kind can't be enchanted, `None` is returned
            pub const fn enchantability(self) -> Option<u8> {
                match self {
                    #item_kind_to_enchantability_arms
                    _ => None
                }
            }

            /// Returns if the item can survive in fire/lava
            #[allow(clippy::match_like_matches_macro)]
            pub const fn fireproof(self) -> bool {
                match self {
                    #item_kind_to_fireproof_arms
                    _ => false
                }
            }

            /// Construct an item kind from a block kind.
            ///
            /// If the given block kind doesn't have a corresponding item kind, `None` is returned.
            pub const fn from_block_kind(block_kind: BlockKind) -> Option<Self> {
                match block_kind {
                    #block_kind_to_item_kind_arms
                    _ => None
                }
            }

            /// Construct a `BlockKindType` from an item kind.
            ///
            /// If the given item kind doesn't have a corresponding block kind, `None` is returned.
            pub const fn from_item_kind(self) -> Option<BlockKindType> {
                match self {
                    #item_kind_to_block_kind_arms
                    _ => None
                }
            }

            /// An array of all item kinds.
            pub const ALL: [Self; #item_kinds_count] = [#(Self::#item_kinds_varients,)*];
        }

        #[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
        /// A struct to store all about the food part of an item
        pub struct FoodComponent {
            hunger: u8,
            saturation: f32,
            always_edible: bool,
            meat: bool,
            snack: bool,
            // TODO: Implement when postions are implemented
            //effects: Vec<()>,
        }
    })
}
