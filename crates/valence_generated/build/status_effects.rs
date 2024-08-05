use heck::ToPascalCase;
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;
use valence_build_utils::{ident, rerun_if_changed};

#[derive(Deserialize, Debug)]
pub(crate) enum StatusEffectCategory {
    Beneficial,
    Harmful,
    Neutral,
}

#[derive(Deserialize, Debug)]
pub(crate) struct AttributeModifiers {
    attribute: u8,
    operation: u8,
    base_value: f64,
    uuid: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct StatusEffect {
    id: u16,
    name: String,
    translation_key: String,
    category: StatusEffectCategory,
    color: u32,
    instant: bool,
    attribute_modifiers: Option<Vec<AttributeModifiers>>,
}

pub(crate) fn build() -> anyhow::Result<TokenStream> {
    rerun_if_changed(["extracted/effects.json"]);

    let effects =
        serde_json::from_str::<Vec<StatusEffect>>(include_str!("../extracted/effects.json"))?;

    let effect_count = effects.len();

    let effect_from_raw_id_arms = effects
        .iter()
        .map(|effect| {
            let id = &effect.id;
            let name = ident(effect.name.to_pascal_case());

            quote! {
                #id => Some(Self::#name),
            }
        })
        .collect::<TokenStream>();

    let effect_to_raw_id_arms = effects
        .iter()
        .map(|effect| {
            let id = &effect.id;
            let name = ident(effect.name.to_pascal_case());

            quote! {
                Self::#name => #id,
            }
        })
        .collect::<TokenStream>();

    let effect_from_ident_arms = effects
        .iter()
        .map(|effect| {
            let path_name = &effect.name;
            let ident_name = format!("minecraft:{}", &effect.name);

            let name = ident(path_name.to_pascal_case());
            quote! {
                #ident_name => Some(Self::#name),
            }
        })
        .collect::<TokenStream>();

    let effect_to_ident_arms = effects
        .iter()
        .map(|effect| {
            let str_name = &effect.name;
            let name = ident(str_name.to_pascal_case());
            quote! {
                Self::#name => ident!(#str_name),
            }
        })
        .collect::<TokenStream>();

    let effect_to_translation_key_arms = effects
        .iter()
        .map(|effect| {
            let str_name = &effect.translation_key;
            let name = ident(effect.name.to_pascal_case());
            quote! {
                Self::#name => #str_name,
            }
        })
        .collect::<TokenStream>();

    let effect_to_category_arms = effects
        .iter()
        .map(|effect| {
            let category = match &effect.category {
                StatusEffectCategory::Beneficial => quote! { StatusEffectCategory::Beneficial },
                StatusEffectCategory::Harmful => quote! { StatusEffectCategory::Harmful },
                StatusEffectCategory::Neutral => quote! { StatusEffectCategory::Neutral },
            };

            let name = ident(effect.name.to_pascal_case());
            quote! {
                Self::#name => #category,
            }
        })
        .collect::<TokenStream>();

    let effect_to_color_arms = effects
        .iter()
        .map(|effect| {
            let color = &effect.color;
            let name = ident(effect.name.to_pascal_case());
            quote! {
                Self::#name => #color,
            }
        })
        .collect::<TokenStream>();

    let effect_to_instant_arms = effects
        .iter()
        .map(|effect| {
            let instant = &effect.instant;
            let name = ident(effect.name.to_pascal_case());
            quote! {
                Self::#name => #instant,
            }
        })
        .collect::<TokenStream>();

    let effect_to_attribute_modifiers_arms = effects
        .iter()
        .filter_map(|effect| {
            effect.attribute_modifiers.as_ref().map(|modifiers| {
                let name = ident(effect.name.to_pascal_case());
                let modifiers = modifiers.iter().map(|modifier| {
                    let attribute = &modifier.attribute;
                    let operation = &modifier.operation;
                    let base_value = &modifier.base_value;
                    let uuid = &modifier.uuid;

                    quote! {
                        AttributeModifier {
                            attribute: EntityAttribute::from_id(#attribute).unwrap(),
                            operation: EntityAttributeOperation::from_raw(#operation).unwrap(),
                            base_value: #base_value,
                            uuid: Uuid::parse_str(#uuid).unwrap(),
                        }
                    }
                });

                quote! {
                    Self::#name => vec![#(#modifiers,)*],
                }
            })
        })
        .collect::<TokenStream>();

    let effect_variants = effects
        .iter()
        .map(|effect| ident(effect.name.to_pascal_case()))
        .collect::<Vec<_>>();

    Ok(quote! {
        use uuid::Uuid;
        use valence_ident::{Ident, ident};
        use super::attributes::{EntityAttribute, EntityAttributeOperation};

        /// Represents an attribute modifier.
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct AttributeModifier {
            /// The attribute that this modifier modifies.
            pub attribute: EntityAttribute,
            /// The operation that this modifier applies.
            pub operation: EntityAttributeOperation,
            /// The base value of this modifier.
            pub base_value: f64,
            /// The UUID of this modifier.
            pub uuid: Uuid,
        }

        /// Represents a status effect category
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub enum StatusEffectCategory {
            Beneficial,
            Harmful,
            Neutral,
        }

        /// Represents a status effect from the game
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub enum StatusEffect {
            #(#effect_variants,)*
        }

        impl StatusEffect {
            /// Constructs a effect from a raw item ID.
            ///
            /// If the given ID is invalid, `None` is returned.
            pub const fn from_raw(id: u16) -> Option<Self> {
                match id {
                    #effect_from_raw_id_arms
                    _ => None
                }
            }

            /// Gets the raw effect ID from the effect
            pub const fn to_raw(self) -> u16 {
                match self {
                    #effect_to_raw_id_arms
                }
            }

            /// Construct a effect from its snake_case name.
            ///
            /// Returns `None` if the name is invalid.
            pub fn from_ident(id: Ident<&str>) -> Option<Self> {
                match id.as_str() {
                    #effect_from_ident_arms
                    _ => None
                }
            }

            /// Gets the identifier of this effect.
            pub const fn to_ident(self) -> Ident<&'static str> {
                match self {
                    #effect_to_ident_arms
                }
            }

            /// Gets the name of this effect.
            /// Same as [`StatusEffect::to_ident`], but doesn't take ownership.
            pub const fn name(&self) -> Ident<&'static str> {
                match self {
                    #effect_to_ident_arms
                }
            }

            /// Gets the translation key of this effect.
            pub const fn translation_key(&self) -> &'static str {
                match self {
                    #effect_to_translation_key_arms
                }
            }

            /// Gets the category of this effect.
            pub const fn category(&self) -> StatusEffectCategory {
                match self {
                    #effect_to_category_arms
                }
            }

            /// Gets the color of this effect.
            pub const fn color(&self) -> u32 {
                match self {
                    #effect_to_color_arms
                }
            }

            /// Gets whether this effect is instant.
            pub const fn instant(&self) -> bool {
                match self {
                    #effect_to_instant_arms
                }
            }

            /// Gets the attribute modifiers of this effect.
            pub fn attribute_modifiers(&self) -> Vec<AttributeModifier> {
                match self {
                    #effect_to_attribute_modifiers_arms
                    _ => vec![],
                }
            }

            /// An array of all effects.
            pub const ALL: [Self; #effect_count] = [#(Self::#effect_variants,)*];
        }
    })
}
