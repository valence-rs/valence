use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse2, parse_quote, Attribute, DeriveInput, Error, Expr, LitInt, Result};

use crate::{add_trait_bounds, decode_split_for_impl};

pub(super) fn derive_packet(item: TokenStream) -> Result<TokenStream> {
    let mut input = parse2::<DeriveInput>(item)?;

    let Some(packet_attr) = parse_packet_helper_attr(&input.attrs)? else {
        return Err(Error::new(input.span(), "missing `packet` attribute"));
    };

    let Some(packet_id) = packet_attr.id else {
        return Err(Error::new(packet_attr.span, "missing `id = ...` value from packet attribute"));
    };

    let lifetime = input
        .generics
        .lifetimes()
        .next()
        .map(|l| l.lifetime.clone())
        .unwrap_or_else(|| parse_quote!('a));

    add_trait_bounds(
        &mut input.generics,
        quote!(::valence_core::__private::Encode),
    );

    add_trait_bounds(
        &mut input.generics,
        quote!(::valence_core::__private::Decode<#lifetime>),
    );

    add_trait_bounds(&mut input.generics, quote!(::std::fmt::Debug));

    let (impl_generics, ty_generics, where_clause) =
        decode_split_for_impl(input.generics, lifetime.clone());

    let name_str = input.ident.to_string();
    let name = input.ident;

    Ok(quote! {
        impl #impl_generics ::valence_core::__private::Packet<#lifetime> for #name #ty_generics
        #where_clause
        {
            const PACKET_ID: i32 = #packet_id;

            fn packet_id(&self) -> i32 {
                #packet_id
            }

            fn packet_name(&self) -> &str {
                #name_str
            }

            fn encode_packet(&self, mut w: impl ::std::io::Write) -> ::valence_core::__private::Result<()> {
                use ::valence_core::__private::{Encode, Context, VarInt};

                VarInt(#packet_id)
                    .encode(&mut w)
                    .context("failed to encode packet ID")?;

                Encode::encode(self, w)
            }

            fn decode_packet(r: &mut &#lifetime [u8]) -> ::valence_core::__private::Result<Self> {
                use ::valence_core::__private::{Decode, Context, VarInt};

                let id = VarInt::decode(r).context("failed to decode packet ID")?.0;
                ::valence_core::__private::ensure!(
                    id == #packet_id, "unexpected packet ID {} (expected {})", id, #packet_id
                );

                Decode::decode(r)
            }
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
