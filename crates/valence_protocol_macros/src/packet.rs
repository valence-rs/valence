use heck::ToShoutySnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse2, parse_quote, Attribute, DeriveInput, Error, Expr, LitInt, LitStr, Result, Type};

use crate::add_trait_bounds;

pub(super) fn derive_packet(item: TokenStream) -> Result<TokenStream> {
    let mut input = parse2::<DeriveInput>(item)?;

    let mut output = TokenStream::new();

    let mut helper_attrs = vec![];

    for attr in &input.attrs {
        if let Some(helper) = parse_packet_helper_attr(attr)? {
            helper_attrs.push(helper);
        }
    }

    if helper_attrs.is_empty() {
        helper_attrs.push(PacketAttr {
            span: Span::call_site(),
            id: Default::default(),
            tag: Default::default(),
            name: Default::default(),
            side: Default::default(),
            state: Default::default(),
        });
    }

    for packet_attr in helper_attrs {
        let type_name = input.ident.clone();

        let name_str = if let Some(attr_name) = packet_attr.name {
            attr_name.value()
        } else {
            type_name.to_string()
        };

        let packet_id: Expr = match packet_attr.id {
            Some(expr) => expr,
            None => match syn::parse_str::<Ident>(&name_str.to_shouty_snake_case()) {
                Ok(ident) => parse_quote!(::valence_protocol::packet::id::#ident),
                Err(_) => {
                    return Err(Error::new(
                        packet_attr.span,
                        "missing valid `id = ...` value from `packet` helper attribute",
                    ))
                }
            },
        };

        add_trait_bounds(&mut input.generics, quote!(::std::fmt::Debug));

        let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

        let side = if let Some(side_attr) = packet_attr.side {
            side_attr
        } else if name_str.to_lowercase().ends_with("s2c") {
            parse_quote!(::valence_protocol::packet::PacketS2c)
        } else if name_str.to_lowercase().ends_with("c2s") {
            parse_quote!(::valence_protocol::packet::PacketC2s)
        } else {
            return Err(Error::new(
                packet_attr.span,
                "missing `side = ...` value from `packet` helper attribute",
            ));
        };

        let state = packet_attr
            .state
            .unwrap_or_else(|| parse_quote!(::valence_protocol::packet::PacketPlay));

        output.extend(quote! {
            impl #impl_generics ::valence_protocol::packet::Packet<#side, #state> for #type_name #ty_generics
            #where_clause
            {
                const ID: i32 = #packet_id;
                const NAME: &'static str = #name_str;
            }
        });
    }

    Ok(output)
}

struct PacketAttr {
    span: Span,
    id: Option<Expr>,
    tag: Option<i32>,
    name: Option<LitStr>,
    side: Option<Type>,
    state: Option<Type>,
}

fn parse_packet_helper_attr(attr: &Attribute) -> Result<Option<PacketAttr>> {
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
                res.side = Some(meta.value()?.parse::<Type>()?);
                Ok(())
            } else if meta.path.is_ident("state") {
                res.state = Some(meta.value()?.parse::<Type>()?);
                Ok(())
            } else {
                Err(meta.error("unrecognized #[packet(...)] argument"))
            }
        })?;

        Ok(Some(res))
    } else {
        Ok(None)
    }
}
