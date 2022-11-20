use anyhow::Ok;
use heck::{ToShoutySnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;

use crate::ident;

#[derive(Deserialize, Clone, Debug)]
struct Translation {
    key: String,
    english_translation: String,
}

/// Escapes characters that have special meaning inside docs.
fn escape(text: &str) -> String {
    text.replace('[', "\\[").replace(']', "\\]")
}

pub fn build_consts() -> anyhow::Result<TokenStream> {
    let translations = serde_json::from_str::<Vec<Translation>>(include_str!(
        "../../extracted/translation_keys.json"
    ))?;

    let translation_key_consts = translations
        .iter()
        .map(|translation| {
            let const_id = ident(translation.key.to_shouty_snake_case());
            let key = &translation.key;
            let english_translation = &translation.english_translation;
            let doc = escape(english_translation);

            quote! {
                #[doc = #doc]
                pub const #const_id: &str = #key;
            }
        })
        .collect::<Vec<TokenStream>>();

    Ok(quote! {
        #(#translation_key_consts)*
    })
}

pub fn build_enum() -> anyhow::Result<TokenStream> {
    let translations = serde_json::from_str::<Vec<Translation>>(include_str!(
        "../../extracted/translation_keys.json"
    ))?;

    let translation_key_variants = translations
        .iter()
        .map(|translation| {
            let variant_id = ident(translation.key.to_upper_camel_case());
            let key = &translation.key;
            let english_translation = &translation.english_translation;
            let doc = format!("{} ({})", escape(key), escape(english_translation));

            quote! {
                #[doc = #doc]
                #variant_id
            }
        })
        .collect::<Vec<TokenStream>>();

    Ok(quote! {
        pub enum TranslationKey {
            #(#translation_key_variants,)*

            /// A custom translation key not available in the bundled language resource pack.
            Custom(String),
        }
    })
}

pub fn build_enum_display() -> anyhow::Result<TokenStream> {
    let translations = serde_json::from_str::<Vec<Translation>>(include_str!(
        "../../extracted/translation_keys.json"
    ))?;

    let translation_key_matches = translations
        .iter()
        .map(|translation| {
            let variant_id = ident(translation.key.to_upper_camel_case());
            let const_id = ident(translation.key.to_shouty_snake_case());

            quote! {
                #variant_id => #const_id
            }
        })
        .collect::<Vec<TokenStream>>();

    Ok(quote! {
        impl TranslationKey {
            pub fn translation_key(&self) -> &str {
                match self {
                    #(Self::#translation_key_matches,)*
                    Self::Custom(key) => key.as_ref(),
                }
            }
        }

        impl std::fmt::Display for TranslationKey {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.translation_key())
            }
        }
    })
}
