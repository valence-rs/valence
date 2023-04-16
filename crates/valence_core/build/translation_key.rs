use anyhow::Ok;
use heck::ToShoutySnakeCase;
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;
use valence_macro_utils::ident;

#[derive(Deserialize, Clone, Debug)]
struct Translation {
    key: String,
    english_translation: String,
}

/// Escapes characters that have special meaning inside docs.
fn escape(text: &str) -> String {
    text.replace('[', "\\[").replace(']', "\\]")
}

pub fn build() -> anyhow::Result<TokenStream> {
    let translations = serde_json::from_str::<Vec<Translation>>(include_str!(
        "../../../extracted/translation_keys.json"
    ))?;

    let translation_key_consts = translations
        .iter()
        .map(|translation| {
            let const_id = ident(translation.key.to_shouty_snake_case());
            let key = &translation.key;
            let english_translation = &translation.english_translation;
            let doc = format!("\"{}\"", escape(english_translation));

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
