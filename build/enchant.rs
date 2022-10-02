use heck::ToPascalCase;
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;

use crate::ident;

#[derive(Deserialize, Debug)]
struct TopLevel {
    enchants: Vec<ParsedEnchantment>,
}

#[derive(Deserialize, Debug)]
pub struct ParsedEnchantment {
    #[allow(unused)]
    id: u16,
    name: String,
    translation_key: String,
    min_level: i16,
    max_level: i16,
    #[serde(alias = "cursed")]
    is_curse: bool,
    rarity_weight: i32,
    #[serde(alias = "sources")]
    source: ParsedEnchantmentSource,
}

#[derive(Deserialize, Debug)]
pub struct ParsedEnchantmentSource {
    treasure: bool,
    enchantment_table: bool,
    random_selection: bool,
}

pub fn build() -> anyhow::Result<TokenStream> {
    let TopLevel { enchants } = serde_json::from_str(include_str!("../extracted/enchants.json"))?;

    let enchantmentkind_definitions = enchants
        .iter()
        .map(|enchant| {
            let rustified_name = ident(enchant.name.to_pascal_case());
            let id = enchant.id as isize;
            quote! {
                #rustified_name = #id,
            }
        })
        .collect::<TokenStream>();

    let enchantmentkind_id_to_variant_lookup = enchants
        .iter()
        .map(|enchant| {
            let rustified_name = ident(enchant.name.to_pascal_case());
            let id = &enchant.id;
            quote! {
                #id => Some(Self::#rustified_name),
            }
        })
        .collect::<TokenStream>();

    let enchantmentkind_ids = enchants
        .iter()
        .map(|enchant| {
            let rustified_name = ident(enchant.name.to_pascal_case());
            let id = &enchant.id;
            quote! {
                Self::#rustified_name => #id,
            }
        })
        .collect::<TokenStream>();

    let enchantmentkind_names = enchants
        .iter()
        .map(|enchant| {
            let rustified_name = ident(enchant.name.to_pascal_case());
            let name = &enchant.name;
            quote! {
                Self::#rustified_name => #name,
            }
        })
        .collect::<TokenStream>();

    let enchantmentkind_translations = enchants
        .iter()
        .map(|enchant| {
            let rustified_name = ident(enchant.name.to_pascal_case());
            let translation_key = &enchant.translation_key;
            quote! {
                Self::#rustified_name => #translation_key,
            }
        })
        .collect::<TokenStream>();

    let enchantmentkind_min_level = enchants
        .iter()
        .map(|enchant| {
            let rustified_name = ident(enchant.name.to_pascal_case());
            let min_level = &enchant.min_level;
            quote! {
                Self::#rustified_name => #min_level,
            }
        })
        .collect::<TokenStream>();

    let enchantmentkind_max_level = enchants
        .iter()
        .map(|enchant| {
            let rustified_name = ident(enchant.name.to_pascal_case());
            let max_level = &enchant.max_level;
            quote! {
                Self::#rustified_name => #max_level,
            }
        })
        .collect::<TokenStream>();

    let enchantmentkind_is_curse = enchants
        .iter()
        .map(|enchant| {
            let rustified_name = ident(enchant.name.to_pascal_case());
            let is_curse = &enchant.is_curse;
            quote! {
                Self::#rustified_name => #is_curse,
            }
        })
        .collect::<TokenStream>();

    let enchantmentkind_rarity_weight = enchants
        .iter()
        .map(|enchant| {
            let rustified_name = ident(enchant.name.to_pascal_case());
            let rarity_weight = &enchant.rarity_weight;
            quote! {
                Self::#rustified_name => #rarity_weight,
            }
        })
        .collect::<TokenStream>();

    let enchantmentkind_source_treasure = enchants
        .iter()
        .map(|enchant| {
            let rustified_name = ident(enchant.name.to_pascal_case());
            let source_treasure = &enchant.source.treasure;
            quote! {
                Self::#rustified_name => #source_treasure,
            }
        })
        .collect::<TokenStream>();

    let enchantmentkind_source_enchantment_table = enchants
        .iter()
        .map(|enchant| {
            let rustified_name = ident(enchant.name.to_pascal_case());
            let enchantment_table = &enchant.source.enchantment_table;
            quote! {
                Self::#rustified_name => #enchantment_table,
            }
        })
        .collect::<TokenStream>();

    let enchantmentkind_source_random_selection = enchants
        .iter()
        .map(|enchant| {
            let rustified_name = ident(enchant.name.to_pascal_case());
            let random_selection = &enchant.source.random_selection;
            quote! {
                Self::#rustified_name => #random_selection,
            }
        })
        .collect::<TokenStream>();

    Ok(quote! {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum EnchantmentKind{
            #enchantmentkind_definitions
        }

        impl EnchantmentKind{
            /// Constructs an `EnchantmentKind` from a raw enchantment ID.
            ///
            /// If the given ID is invalid, `None` is returned.
            pub const fn from_raw(id: u16) -> Option<Self> {
                match id{
                    #enchantmentkind_id_to_variant_lookup
                    _ => None
                }
            }

            /// Returns the enchantment ID.
            pub const fn id(&self) -> u16{
                match &self{
                    #enchantmentkind_ids
                }
            }

            /// Returns the translation key.
            pub const fn translation_key(&self) -> &'static str{
                match &self{
                    #enchantmentkind_translations
                }
            }

            /// Returns the enchantment name the game uses.
            pub const fn name(&self) -> &'static str{
                match &self{
                    #enchantmentkind_names
                }
            }

            /// Returns the minimum enchantment level officially supported by Minecraft.
            pub const fn min_level(&self) -> i16{
                match &self{
                    #enchantmentkind_min_level
                }
            }

            /// Returns the maximum enchantment level officially supported by Minecraft.
            pub const fn max_level(&self) -> i16{
                match &self{
                    #enchantmentkind_max_level
                }
            }

            /// Returns true if the enchantment is of the curse type.
            pub const fn is_curse(&self) -> bool{
                match &self{
                    #enchantmentkind_is_curse
                }
            }

            /// Returns the rarity of the enchant. Lower means more rare.
            pub const fn rarity_weight(&self) -> i32{
                match &self{
                    #enchantmentkind_rarity_weight
                }
            }

            /// Returns true if the enchantment is of a treasure type.
            pub const fn is_treasure_source(&self) -> bool{
                match &self{
                    #enchantmentkind_source_treasure
                }
            }

            /// Returns true if the enchantment can be obtained through a enchantment table.
            pub const fn is_enchantment_table_source(&self) -> bool{
                match &self{
                    #enchantmentkind_source_enchantment_table
                }
            }

            /// Returns true if the enchantment can be chosen randomly by using an enchantment table, for example.
            pub const fn is_random_selection_source(&self) -> bool{
                match &self{
                    #enchantmentkind_source_random_selection
                }
            }
        }
    })
}
