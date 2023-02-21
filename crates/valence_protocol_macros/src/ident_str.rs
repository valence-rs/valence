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
    let mut ident = &ident_lit.value()[..];

    let path_start = match ident.split_once(':') {
        Some(("minecraft", path)) if check_path(path) => {
            ident = path;
            0
        }
        Some((namespace, path)) if check_namespace(namespace) && check_path(path) => {
            namespace.len() + 1
        }
        None if check_path(ident) => 0,
        _ => {
            return Err(syn::Error::new(
                ident_lit.span(),
                "string cannot be parsed as ident",
            ))
        }
    };

    Ok(quote! {
        ::valence_protocol::ident::Ident::new_unchecked(#ident, #path_start)
    })
}
