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

    let enchant_impls = enchants
        .iter()
        .map(|enchant| {
            let enchantment_variant = ident({
                let mut name = enchant.name.to_pascal_case();
                name.push_str("Enchantment");
                name
            });
            let translation_key = &enchant.translation_key;
            let name = &enchant.name;
            let id = &enchant.id;
            let min_level = &enchant.min_level;
            let max_level = &enchant.max_level;
            let is_curse = &enchant.is_curse;
            let rarity_weight = &enchant.rarity_weight;
            let source_treasure = &enchant.source.treasure;
            let source_enchantment_table = &enchant.source.enchantment_table;
            let source_random_selection = &enchant.source.random_selection;

            let level_field = if min_level != max_level {
                let level_notice =
                    format!("Vanilla minecraft supports levels {min_level}..={max_level}");
                quote! {
                    #[doc = "The level of this enchantment."]
                    #[doc = #level_notice]
                    #[allow(unused)]
                    level: i16
                }
            } else {
                let warning = format!(
                    "Warning: Vanilla minecraft only supports level {}.",
                    min_level
                );

                quote! {
                    #[doc = "The level of this enchantment."]
                    #[doc = #warning ]
                    #[allow(unused)]
                    level: i16
                }
            };

            quote! {
                #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
                pub struct #enchantment_variant{
                    #level_field
                }

                impl EnchantmentDescriptor for #enchantment_variant {
                    const ID: u16 = #id;
                    const NAME: &'static str = #name;
                    const TRANSLATION_KEY: &'static str = #translation_key;
                    const MIN_LEVEL: i16 = #min_level;
                    const MAX_LEVEL: i16 = #max_level;
                    const IS_CURSE: bool = #is_curse;
                    const RARITY_WEIGHT: i32 = #rarity_weight;
                }

                impl EnchantmentSource for #enchantment_variant {
                    const TREASURE: bool = #source_treasure;
                    const ENCHANTMENT_TABLE: bool = #source_enchantment_table;
                    const RANDOM_SELECTION: bool = #source_random_selection;
                }
            }
        })
        .collect::<TokenStream>();

    let enum_definition = enchants
        .iter()
        .map(|enchant| {
            let name_long = ident({
                let mut name = enchant.name.to_pascal_case();
                name.push_str("Enchantment");
                name
            });
            let name_short = ident(enchant.name.to_pascal_case());
            quote! {
                #name_short ( #name_long ),
            }
        })
        .collect::<TokenStream>();

    Ok(quote! {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum EnchantmentKind{
            #enum_definition
        }

        pub trait EnchantmentDescriptor: EnchantmentSource {
            const ID: u16;
            const NAME: &'static str;
            const TRANSLATION_KEY: &'static str;
            const MIN_LEVEL: i16;
            const MAX_LEVEL: i16;
            const IS_CURSE: bool;
            const RARITY_WEIGHT: i32;
        }

        pub trait EnchantmentSource {
            const TREASURE: bool;
            const ENCHANTMENT_TABLE: bool;
            const RANDOM_SELECTION: bool;
        }

        #enchant_impls
    })
}
