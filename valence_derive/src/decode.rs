use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{parse2, parse_quote, Data, DeriveInput, Error, Fields, Result};

use crate::{
    add_trait_bounds, decode_split_for_impl, find_packet_id_attr, pair_variants_with_discriminants,
};

pub fn derive_decode(item: TokenStream) -> Result<TokenStream> {
    let mut input = parse2::<DeriveInput>(item)?;

    let name = input.ident;
    let string_name = name.to_string();

    let packet_id = find_packet_id_attr(&input.attrs)?
        .into_iter()
        .map(|l| l.to_token_stream())
        .collect::<Vec<_>>();

    if input.generics.lifetimes().count() > 1 {
        return Err(Error::new(
            input.generics.params.span(),
            "type deriving `Decode` must have no more than one lifetime",
        ));
    }

    // Use the lifetime specified in the type definition or just use `'a` if not
    // present.
    let lifetime = input
        .generics
        .lifetimes()
        .next()
        .map(|l| l.lifetime.clone())
        .unwrap_or_else(|| parse_quote!('a));

    match input.data {
        Data::Struct(struct_) => {
            let decode_fields = match struct_.fields {
                Fields::Named(fields) => {
                    let init = fields.named.iter().map(|f| {
                        let name = f.ident.as_ref().unwrap();
                        let ctx = format!("failed to decode field `{name}`");
                        quote! {
                            #name: Decode::decode(_r).context(#ctx)?,
                        }
                    });

                    quote! {
                        Self {
                            #(#init)*
                        }
                    }
                }
                Fields::Unnamed(fields) => {
                    let init = (0..fields.unnamed.len())
                        .map(|i| {
                            let ctx = format!("failed to decode field `{i}`");
                            quote! {
                                Decode::decode(_r).context(#ctx)?,
                            }
                        })
                        .collect::<TokenStream>();

                    quote! {
                        Self(#init)
                    }
                }
                Fields::Unit => quote!(Self),
            };

            add_trait_bounds(
                &mut input.generics,
                quote!(::valence_protocol::Decode<#lifetime>),
            );

            let (impl_generics, ty_generics, where_clause) =
                decode_split_for_impl(input.generics, lifetime.clone());

            Ok(quote! {
                #[allow(unused_imports)]
                impl #impl_generics ::valence_protocol::Decode<#lifetime> for #name #ty_generics
                #where_clause
                {
                    fn decode(_r: &mut &#lifetime [u8]) -> ::valence_protocol::__private::Result<Self> {
                        use ::valence_protocol::__private::{Decode, Context, VarInt, ensure};

                        #(
                            let id = VarInt::decode(_r).context("failed to decode packet ID")?.0;
                            ensure!(id == #packet_id, "unexpected packet ID {} (expected {})", id, #packet_id);
                        )*

                        Ok(#decode_fields)
                    }
                }

                #(
                    #[allow(unused_imports)]
                    impl #impl_generics ::valence_protocol::DerivedPacketDecode<#lifetime> for #name #ty_generics
                    #where_clause
                    {
                        const ID: i32 = #packet_id;
                        const NAME: &'static str = #string_name;

                        fn decode_without_id(_r: &mut &#lifetime [u8]) -> ::valence_protocol::__private::Result<Self> {
                            use ::valence_protocol::__private::{Decode, Context, VarInt, ensure};

                            Ok(#decode_fields)
                        }
                    }
                )*
            })
        }
        Data::Enum(enum_) => {
            let variants = pair_variants_with_discriminants(enum_.variants.into_iter())?;

            let decode_arms = variants
                .iter()
                .map(|(disc, variant)| {
                    let name = &variant.ident;

                    match &variant.fields {
                        Fields::Named(fields) => {
                            let fields = fields
                                .named
                                .iter()
                                .map(|f| {
                                    let field = f.ident.as_ref().unwrap();
                                    let ctx = format!(
                                        "failed to decode field `{field}` in variant `{name}`",
                                    );
                                    quote! {
                                        #field: Decode::decode(_r).context(#ctx)?,
                                    }
                                })
                                .collect::<TokenStream>();

                            quote! {
                                #disc => Ok(Self::#name { #fields }),
                            }
                        }
                        Fields::Unnamed(fields) => {
                            let init = (0..fields.unnamed.len())
                                .map(|i| {
                                    let ctx = format!(
                                        "failed to decode field `{i}` in variant `{name}`",
                                    );
                                    quote! {
                                        Decode::decode(_r).context(#ctx)?,
                                    }
                                })
                                .collect::<TokenStream>();

                            quote! {
                                #disc => Ok(Self::#name(#init)),
                            }
                        }
                        Fields::Unit => quote!(#disc => Ok(Self::#name),),
                    }
                })
                .collect::<TokenStream>();

            add_trait_bounds(
                &mut input.generics,
                quote!(::valence_protocol::Decode<#lifetime>),
            );

            let (impl_generics, ty_generics, where_clause) =
                decode_split_for_impl(input.generics, lifetime.clone());

            Ok(quote! {
                #[allow(unused_imports)]
                impl #impl_generics ::valence_protocol::Decode<#lifetime> for #name #ty_generics
                #where_clause
                {
                    fn decode(_r: &mut &#lifetime [u8]) -> ::valence_protocol::__private::Result<Self> {
                        use ::valence_protocol::__private::{Decode, Context, VarInt, bail, ensure};

                        #(
                            let id = VarInt::decode(_r).context("failed to decode packet ID")?.0;
                            ensure!(id == #packet_id, "unexpected packet ID {} (expected {})", id, #packet_id);
                        )*

                        let disc = VarInt::decode(_r).context("failed to decode enum discriminant")?.0;
                        match disc {
                            #decode_arms
                            n => bail!("unexpected enum discriminant {}", disc),
                        }
                    }
                }

                #(
                    #[allow(unused_imports)]
                    impl #impl_generics ::valence_protocol::DerivedPacketDecode<#lifetime> for #name #ty_generics
                    #where_clause
                    {
                        const ID: i32 = #packet_id;
                        const NAME: &'static str = #string_name;

                        fn decode_without_id(_r: &mut &#lifetime [u8]) -> ::valence_protocol::__private::Result<Self> {
                            use ::valence_protocol::__private::{Decode, Context, VarInt, bail};

                            let disc = VarInt::decode(_r).context("failed to decode enum discriminant")?.0;
                            match disc {
                                #decode_arms
                                n => bail!("unexpected enum discriminant {}", disc),
                            }
                        }
                    }
                )*
            })
        }
        Data::Union(u) => Err(Error::new(
            u.union_token.span(),
            "cannot derive `Decode` on unions",
        )),
    }
}
