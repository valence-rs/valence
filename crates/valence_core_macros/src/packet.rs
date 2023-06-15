use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse2, Attribute, DeriveInput, Error, Expr, LitInt, Result};

use crate::add_trait_bounds;

pub(super) fn derive_packet(item: TokenStream) -> Result<TokenStream> {
    let mut input = parse2::<DeriveInput>(item)?;

    let Some(packet_attr) = parse_packet_helper_attr(&input.attrs)? else {
        return Err(Error::new(input.span(), "missing `packet` attribute"));
    };

    let Some(packet_id) = packet_attr.id else {
        return Err(Error::new(packet_attr.span, "missing `id = ...` value from packet attribute"));
    };

    add_trait_bounds(&mut input.generics, quote!(::std::fmt::Debug));

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let name_str = input.ident.to_string();
    let name = input.ident;

    Ok(quote! {
        impl #impl_generics ::valence_core::__private::Packet for #name #ty_generics
        #where_clause
        {
            const ID: i32 = #packet_id;
            const NAME: &'static str = #name_str;
        }
    })
}

struct PacketAttr {
    span: Span,
    id: Option<Expr>,
    tag: Option<i32>,
}

fn parse_packet_helper_attr(attrs: &[Attribute]) -> Result<Option<PacketAttr>> {
    for attr in attrs {
        if attr.path().is_ident("packet") {
            let mut res = PacketAttr {
                span: attr.span(),
                id: None,
                tag: None,
            };

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("id") {
                    res.id = Some(meta.value()?.parse::<Expr>()?);
                    Ok(())
                } else if meta.path.is_ident("tag") {
                    res.tag = Some(meta.value()?.parse::<LitInt>()?.base10_parse::<i32>()?);
                    Ok(())
                } else {
                    Err(meta.error("unrecognized packet argument"))
                }
            })?;

            return Ok(Some(res));
        }
    }

    Ok(None)
}
