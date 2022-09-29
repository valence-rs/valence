use anyhow::Ok;
use heck::ToPascalCase;
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;

use crate::ident;

#[derive(Deserialize, Clone, Debug)]
struct Item {
    #[allow(unused)]
    id: u16,
    translation_key: String,
    name: String,
    max_stack: u8,
}

pub fn build() -> anyhow::Result<TokenStream> {
    let items = serde_json::from_str::<Vec<Item>>(include_str!("../extracted/items.json"))?;

    let items_count = items.len();

    let item_to_translation_key_arms = items
        .iter()
        .map(|i| {
            let item = ident(i.name.to_pascal_case());
            let translation_key = &i.translation_key;
            quote! {
                Self::#item => #translation_key,
            }
        })
        .collect::<TokenStream>();

    let item_variants = items
        .iter()
        .map(|i| ident(i.name.to_pascal_case()))
        .collect::<Vec<_>>();

    let item_from_str_arms = items
        .iter()
        .map(|i| {
            let name = &i.name;
            let name_ident = ident(name.to_pascal_case());
            quote! {
                #name => Some(Item::#name_ident),
            }
        })
        .collect::<TokenStream>();

    let item_to_str_arms = items
        .iter()
        .map(|i| {
            let name = &i.name;
            let name_ident = ident(name.to_pascal_case());
            quote! {
                Item::#name_ident => #name,
            }
        })
        .collect::<TokenStream>();

    let item_to_raw_id_arms = items
        .iter()
        .map(|i| {
            let id = &i.id;
            let name_ident = ident(&i.name.to_pascal_case());

            quote! {
                Item::#name_ident => #id,
            }
        })
        .collect::<TokenStream>();

    let item_from_raw_id_arms = items
        .iter()
        .map(|i| {
            let id = &i.id;
            let name_ident = ident(&i.name.to_pascal_case());

            quote! {
                #id => Some(Item::#name_ident),
            }
        })
        .collect::<TokenStream>();

    let item_to_max_stack_arms = items
        .iter()
        .map(|i| {
            let name_ident = ident(&i.name.to_pascal_case());
            let max_count = i.max_stack;

            quote! {
                Item::#name_ident => #max_count,
            }
        })
        .collect::<TokenStream>();

    let items_varients = items
        .iter()
        .map(|i| ident(i.name.to_pascal_case()))
        .collect::<Vec<_>>();

    Ok(quote! {
        /// Represents an item from game
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        #[repr(u16)]
        pub enum Item {
            #(#item_variants,)*
        }

        impl Item {
            /// Constructs a item from a raw item ID.
            ///
            /// If the given ID is invalid, `None` is returned.
            pub const fn from_raw(id: u16) -> Option<Self> {
                match id {
                    #item_from_raw_id_arms
                    _ => None
                }
            }

            /// Gets the raw item ID from the item
            pub const fn to_raw(self) -> u16 {
                match self {
                    #item_to_raw_id_arms
                }
            }

            /// Construct an item for its snake_case name.
            ///
            /// Returns `None` if the name is invalid.
            // Added to make it the same as BlockKind
            #[allow(clippy::should_implement_trait)]
            pub fn from_str(name: &str) -> Option<Item> {
                match name {
                    #item_from_str_arms
                    _ => None
                }
            }

            /// Gets the snake_case name of this item.
            pub const fn to_str(self) -> &'static str {
                match self {
                    #item_to_str_arms
                }
            }

            /// Gets the translation key of an item.
            pub const fn translation_key(self) -> &'static str {
                match self {
                    #item_to_translation_key_arms
                }
            }

            /// Returns the max stack count
            pub const fn max_stack(self) -> u8 {
                match self {
                    #item_to_max_stack_arms
                }
            }

            /// An array of all items.
            pub const ALL: [Self; #items_count] = [#(Self::#items_varients,)*];
        }
    })
}
