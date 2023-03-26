use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, LitStr, Result};

fn check_namespace(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '.' | '-'))
}

fn check_path(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '.' | '-' | '/'))
}

pub fn ident_str(item: TokenStream) -> Result<TokenStream> {
    let ident_lit: LitStr = parse2(item)?;
    let mut ident = ident_lit.value();

    match ident.split_once(':') {
        Some((namespace, path)) if check_namespace(namespace) && check_path(path) => {}
        None if check_path(&ident) => {
            ident = format!("minecraft:{ident}");
        }
        _ => {
            return Err(syn::Error::new(
                ident_lit.span(),
                "string cannot be parsed as a resource identifier",
            ))
        }
    }

    Ok(quote! {
        ::valence_protocol::ident::Ident::new_unchecked(#ident)
    })
}
