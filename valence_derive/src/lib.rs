use proc_macro::TokenStream as StdTokenStream;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse2, parse_quote, Attribute, DeriveInput, Error, GenericParam, Generics, Lifetime,
    LifetimeDef, Lit, LitInt, Meta, Result, Variant,
};

mod decode;
mod encode;

#[proc_macro_derive(Encode, attributes(packet_id, tag))]
pub fn derive_encode(item: StdTokenStream) -> StdTokenStream {
    match encode::derive_encode(item.into()) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

#[proc_macro_derive(Decode, attributes(packet_id, tag))]
pub fn derive_decode(item: StdTokenStream) -> StdTokenStream {
    match decode::derive_decode(item.into()) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

#[proc_macro_derive(Packet)]
pub fn derive_packet(item: StdTokenStream) -> StdTokenStream {
    match derive_packet_inner(item.into()) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

fn derive_packet_inner(item: TokenStream) -> Result<TokenStream> {
    let input = parse2::<DeriveInput>(item)?;

    if find_packet_id_attr(&input.attrs)?.is_none() {
        return Err(Error::new(
            Span::call_site(),
            "cannot derive `Packet` without `#[packet_id = ...]` attribute. Consider implementing \
             the trait manually",
        ));
    }

    let name = input.ident;
    let string_name = name.to_string();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::valence_protocol::Packet for #name #ty_generics
        #where_clause
        {
            fn packet_name(&self) -> &'static str {
                #string_name
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

fn pair_variants_with_discriminants(
    variants: impl IntoIterator<Item = Variant>,
) -> Result<Vec<(i32, Variant)>> {
    let mut discriminant = 0;
    variants
        .into_iter()
        .map(|v| {
            if let Some(i) = find_tag_attr(&v.attrs)? {
                discriminant = i;
            }

            let pair = (discriminant, v);
            discriminant += 1;
            Ok(pair)
        })
        .collect::<Result<_>>()
}

fn find_tag_attr(attrs: &[Attribute]) -> Result<Option<i32>> {
    for attr in attrs {
        if let Meta::NameValue(nv) = attr.parse_meta()? {
            if nv.path.is_ident("tag") {
                let span = nv.lit.span();
                return match nv.lit {
                    Lit::Int(lit) => Ok(Some(lit.base10_parse::<i32>()?)),
                    _ => Err(Error::new(
                        span,
                        "discriminant value must be an integer literal",
                    )),
                };
            }
        }
    }

    Ok(None)
}

/// Adding our lifetime to the generics before calling `.split_for_impl()` would
/// also add it to the resulting ty_generics, which we don't want. So I'm doing
/// this hack.
fn decode_split_for_impl(
    mut generics: Generics,
    lifetime: Lifetime,
) -> (TokenStream, TokenStream, TokenStream) {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut impl_generics = impl_generics.to_token_stream();
    let ty_generics = ty_generics.to_token_stream();
    let where_clause = where_clause.to_token_stream();

    if generics.lifetimes().next().is_none() {
        generics
            .params
            .push(GenericParam::Lifetime(LifetimeDef::new(lifetime)));

        impl_generics = generics.split_for_impl().0.to_token_stream();
    }

    (impl_generics, ty_generics, where_clause)
}

fn add_trait_bounds(generics: &mut Generics, trait_: TokenStream) {
    for param in &mut generics.params {
        if let GenericParam::Type(type_param) = param {
            type_param.bounds.push(parse_quote!(#trait_))
        }
    }
}
