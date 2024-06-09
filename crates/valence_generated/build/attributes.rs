use std::collections::BTreeMap;

use heck::ToPascalCase;
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;
use valence_build_utils::{ident, rerun_if_changed};

#[derive(Deserialize)]
struct EntityAttribute {
    id: u8,
    default_value: f64,
    translation_key: String,
    tracked: bool,
    min_value: f64,
    max_value: f64,
}

pub(crate) fn build() -> anyhow::Result<TokenStream> {
    rerun_if_changed(["extracted/attributes.json"]);

    let entity_attributes: BTreeMap<String, EntityAttribute> =
        serde_json::from_str(include_str!("../extracted/attributes.json"))?;

    let mut entity_attribute_enum = TokenStream::new();
    let mut entity_attribute_get_id = TokenStream::new();
    let mut entity_attribute_from_id = TokenStream::new();
    let mut entity_attribute_name = TokenStream::new();
    let mut entity_attribute_default_value = TokenStream::new();
    let mut entity_attribute_translation_key = TokenStream::new();
    let mut entity_attribute_tracked = TokenStream::new();
    let mut entity_attribute_min_value = TokenStream::new();
    let mut entity_attribute_max_value = TokenStream::new();

    for (name, attribute) in entity_attributes {
        let key = ident(name.to_pascal_case());
        let id = attribute.id;
        let default_value = attribute.default_value;
        let translation_key = attribute.translation_key;
        let tracked = attribute.tracked;
        let min_value = attribute.min_value;
        let max_value = attribute.max_value;

        entity_attribute_enum.extend([quote! {
            #key,
        }]);

        entity_attribute_get_id.extend([quote! {
            EntityAttribute::#key => #id,
        }]);

        entity_attribute_from_id.extend([quote! {
            #id => Some(EntityAttribute::#key),
        }]);

        entity_attribute_name.extend([quote! {
            EntityAttribute::#key => #name,
        }]);

        entity_attribute_default_value.extend([quote! {
            EntityAttribute::#key => #default_value,
        }]);

        entity_attribute_translation_key.extend([quote! {
            EntityAttribute::#key => #translation_key,
        }]);

        entity_attribute_tracked.extend([quote! {
            EntityAttribute::#key => #tracked,
        }]);

        entity_attribute_min_value.extend([quote! {
            EntityAttribute::#key => #min_value,
        }]);

        entity_attribute_max_value.extend([quote! {
            EntityAttribute::#key => #max_value,
        }]);
    }

    Ok(quote!(
        /// An attribute modifier operation.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum EntityAttributeOperation {
            /// Adds the modifier to the base value.
            Add,
            /// Multiplies the modifier with the base value.
            MultiplyBase,
            /// Multiplies the modifier with the total value.
            MultiplyTotal,
        }

        impl EntityAttributeOperation {
            /// Converts from a raw [`u8`].
            pub fn from_raw(raw: u8) -> Option<Self> {
                match raw {
                    0 => Some(Self::Add),
                    1 => Some(Self::MultiplyBase),
                    2 => Some(Self::MultiplyTotal),
                    _ => None,
                }
            }

            /// Converts to a raw [`u8`].
            pub fn to_raw(self) -> u8 {
                match self {
                    Self::Add => 0,
                    Self::MultiplyBase => 1,
                    Self::MultiplyTotal => 2,
                }
            }
        }

        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub enum EntityAttribute {
            #entity_attribute_enum
        }

        impl EntityAttribute {
            pub fn get_id(self) -> u8 {
                match self {
                    #entity_attribute_get_id
                }
            }

            pub fn from_id(id: u8) -> Option<Self> {
                match id {
                    #entity_attribute_from_id
                    _ => None,
                }
            }

            pub fn name(self) -> &'static str {
                match self {
                    #entity_attribute_name
                }
            }

            pub fn default_value(self) -> f64 {
                match self {
                    #entity_attribute_default_value
                }
            }

            pub fn translation_key(self) -> &'static str {
                match self {
                    #entity_attribute_translation_key
                }
            }

            pub fn tracked(self) -> bool {
                match self {
                    #entity_attribute_tracked
                }
            }

            pub fn min_value(self) -> f64 {
                match self {
                    #entity_attribute_min_value
                }
            }

            pub fn max_value(self) -> f64 {
                match self {
                    #entity_attribute_max_value
                }
            }
        }
    ))
}
