use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse2, parse_quote, Attribute, DeriveInput, Error, Expr, LitInt, LitStr, Path, Result};

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

    let name = input.ident.clone();

    let name_str = if let Some(attr_name) = packet_attr.name {
        attr_name.value()
    } else {
        name.to_string()
    };

    let side = if let Some(side_attr) = packet_attr.side {
        side_attr
    } else if name_str.to_lowercase().contains("s2c") {
        parse_quote!(PacketSide::Clientbound)
    } else if name_str.to_lowercase().contains("c2s") {
        parse_quote!(PacketSide::Serverbound)
    } else {
        return Err(Error::new(
            input.span(),
            "missing `side = PacketSide::...` value from packet attribute",
        ));
    };

    let state = packet_attr
        .state
        .unwrap_or_else(|| parse_quote!(PacketState::Play));

    Ok(quote! {
        impl #impl_generics ::valence_core::__private::Packet for #name #ty_generics
        #where_clause
        {
            const ID: i32 = #packet_id;
            const NAME: &'static str = #name_str;
            const SIDE: ::valence_core::protocol::PacketSide = ::valence_core::protocol::#side;
            const STATE: ::valence_core::protocol::PacketState = ::valence_core::protocol::#state;
        }
    })
}

struct PacketAttr {
    span: Span,
    id: Option<Expr>,
    tag: Option<i32>,
    name: Option<LitStr>,
    side: Option<Path>,
    state: Option<Path>,
}

fn parse_packet_helper_attr(attrs: &[Attribute]) -> Result<Option<PacketAttr>> {
    for attr in attrs {
        if attr.path().is_ident("packet") {
            let mut res = PacketAttr {
                span: attr.span(),
                id: None,
                tag: None,
                name: None,
                side: None,
                state: None,
            };

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("id") {
                    res.id = Some(meta.value()?.parse::<Expr>()?);
                    Ok(())
                } else if meta.path.is_ident("tag") {
                    res.tag = Some(meta.value()?.parse::<LitInt>()?.base10_parse::<i32>()?);
                    Ok(())
                } else if meta.path.is_ident("name") {
                    res.name = Some(meta.value()?.parse::<LitStr>()?);
                    Ok(())
                } else if meta.path.is_ident("side") {
                    let path = meta.value()?.parse::<Path>()?;
                    let Some(first) = path.segments.first() else {
                        return Err(meta.error("side path should have length equals 2"));
                    };

                    if first.ident != "PacketSide" {
                        return Err(meta.error("side must starts with `PacketSide`"));
                    }

                    res.side = Some(path);

                    Ok(())
                } else if meta.path.is_ident("state") {
                    let path = meta.value()?.parse::<Path>()?;
                    let Some(first) = path.segments.first() else {
                        return Err(meta.error("state path should have length equals 2"));
                    };

                    if first.ident != "PacketState" {
                        return Err(meta.error("state must starts with `PacketState`"));
                    }

                    res.state = Some(path);
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
