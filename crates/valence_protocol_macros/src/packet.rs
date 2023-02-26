use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, parse_quote, Attribute, DeriveInput, Error, Lit, LitInt, Meta, Result};

use crate::{add_trait_bounds, decode_split_for_impl};

pub fn derive_packet(item: TokenStream) -> Result<TokenStream> {
    let mut input = parse2::<DeriveInput>(item)?;

    let Some(packet_id) = find_packet_id_attr(&input.attrs)? else {
        return Err(Error::new(
            input.ident.span(),
            "cannot derive `Packet` without `#[packet_id = ...]` helper attribute",
        ))
    };

    let lifetime = input
        .generics
        .lifetimes()
        .next()
        .map(|l| l.lifetime.clone())
        .unwrap_or_else(|| parse_quote!('a));

    add_trait_bounds(
        &mut input.generics,
        quote!(::valence_protocol::__private::Encode),
    );

    add_trait_bounds(
        &mut input.generics,
        quote!(::valence_protocol::__private::Decode<#lifetime>),
    );

    add_trait_bounds(&mut input.generics, quote!(::std::fmt::Debug));

    let (impl_generics, ty_generics, where_clause) =
        decode_split_for_impl(input.generics, lifetime.clone());

    let name = input.ident;

    Ok(quote! {
        impl #impl_generics ::valence_protocol::__private::Packet<#lifetime> for #name #ty_generics
        #where_clause
        {
            const PACKET_ID: i32 = #packet_id;

            fn packet_id(&self) -> i32 {
                #packet_id
            }

            fn encode_packet(&self, mut w: impl ::std::io::Write) -> ::valence_protocol::__private::Result<()> {
                use ::valence_protocol::__private::{Encode, Context, VarInt};

                VarInt(#packet_id)
                    .encode(&mut w)
                    .context("failed to encode packet ID")?;

                self.encode(w)
            }

            fn decode_packet(r: &mut &#lifetime [u8]) -> ::valence_protocol::__private::Result<Self> {
                use ::valence_protocol::__private::{Decode, Context, VarInt, ensure};

                let id = VarInt::decode(r).context("failed to decode packet ID")?.0;
                ensure!(id == #packet_id, "unexpected packet ID {} (expected {})", id, #packet_id);

                Self::decode(r)
            }
        }
    })
}

fn find_packet_id_attr(attrs: &[Attribute]) -> Result<Option<LitInt>> {
    for attr in attrs {
        if let Meta::NameValue(nv) = attr.parse_meta()? {
            if nv.path.is_ident("packet_id") {
                let span = nv.lit.span();
                return match nv.lit {
                    Lit::Int(i) => Ok(Some(i)),
                    _ => Err(Error::new(span, "packet ID must be an integer literal")),
                };
            }
        }
    }

    Ok(None)
}
