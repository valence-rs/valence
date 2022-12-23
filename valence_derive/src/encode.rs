use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse2, Data, DeriveInput, Error, Fields, LitInt, Result};

use crate::{add_trait_bounds, find_packet_id_attr, pair_variants_with_discriminants};

pub fn derive_encode(item: TokenStream) -> Result<TokenStream> {
    let mut input = parse2::<DeriveInput>(item)?;

    let name = input.ident;

    add_trait_bounds(
        &mut input.generics,
        quote!(::valence_protocol::__private::Encode),
    );

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    match input.data {
        Data::Struct(struct_) => {
            let encode_fields = match &struct_.fields {
                Fields::Named(fields) => fields
                    .named
                    .iter()
                    .map(|f| {
                        let name = &f.ident.as_ref().unwrap();
                        let ctx = format!("failed to encode field `{name}`");
                        quote! {
                            self.#name.encode(&mut _w).context(#ctx)?;
                        }
                    })
                    .collect(),
                Fields::Unnamed(fields) => (0..fields.unnamed.len())
                    .map(|i| {
                        let lit = LitInt::new(&i.to_string(), Span::call_site());
                        let ctx = format!("failed to encode field `{lit}`");
                        quote! {
                            self.#lit.encode(&mut _w).context(#ctx)?;
                        }
                    })
                    .collect(),
                Fields::Unit => TokenStream::new(),
            };

            Ok(quote! {
                #[allow(unused_imports)]
                impl #impl_generics ::valence_protocol::__private::Encode for #name #ty_generics
                #where_clause
                {
                    fn encode(&self, mut _w: impl ::std::io::Write) -> ::valence_protocol::__private::Result<()> {
                        use ::valence_protocol::__private::{Encode, Context};

                        #encode_fields

                        Ok(())
                    }
                }
            })
        }
        Data::Enum(enum_) => {
            let variants = pair_variants_with_discriminants(enum_.variants.into_iter())?;

            let encode_arms = variants
                .iter()
                .map(|(disc, variant)| {
                    let variant_name = &variant.ident;

                    let disc_ctx = format!(
                        "failed to encode enum discriminant {disc} for variant `{variant_name}`",
                    );

                    match &variant.fields {
                        Fields::Named(fields) => {
                            let field_names = fields
                                .named
                                .iter()
                                .map(|f| f.ident.as_ref().unwrap())
                                .collect::<Vec<_>>();

                            let encode_fields = field_names
                                .iter()
                                .map(|name| {
                                    let ctx = format!(
                                        "failed to encode field `{name}` in variant \
                                         `{variant_name}`",
                                    );

                                    quote! {
                                        #name.encode(&mut _w).context(#ctx)?;
                                    }
                                })
                                .collect::<TokenStream>();

                            quote! {
                                Self::#variant_name { #(#field_names,)* } => {
                                    VarInt(#disc).encode(&mut _w).context(#disc_ctx)?;

                                    #encode_fields
                                    Ok(())
                                }
                            }
                        }
                        Fields::Unnamed(fields) => {
                            let field_names = (0..fields.unnamed.len())
                                .map(|i| Ident::new(&format!("_{i}"), Span::call_site()))
                                .collect::<Vec<_>>();

                            let encode_fields = field_names
                                .iter()
                                .map(|name| {
                                    let ctx = format!(
                                        "failed to encode field `{name}` in variant \
                                         `{variant_name}`"
                                    );

                                    quote! {
                                        #name.encode(&mut _w).context(#ctx)?;
                                    }
                                })
                                .collect::<TokenStream>();

                            quote! {
                                Self::#variant_name(#(#field_names,)*) => {
                                    VarInt(#disc).encode(&mut _w).context(#disc_ctx)?;

                                    #encode_fields
                                    Ok(())
                                }
                            }
                        }
                        Fields::Unit => quote! {
                            Self::#variant_name => Ok(
                                VarInt(#disc)
                                    .encode(&mut _w)
                                    .context(#disc_ctx)?
                            ),
                        },
                    }
                })
                .collect::<TokenStream>();

            Ok(quote! {
                #[allow(unused_imports, unreachable_code)]
                impl #impl_generics ::valence_protocol::Encode for #name #ty_generics
                #where_clause
                {
                    fn encode(&self, mut _w: impl ::std::io::Write) -> ::valence_protocol::__private::Result<()> {
                        use ::valence_protocol::__private::{Encode, VarInt, Context};

                        match self {
                            #encode_arms
                            _ => unreachable!(),
                        }
                    }
                }
            })
        }
        Data::Union(u) => Err(Error::new(
            u.union_token.span(),
            "cannot derive `Encode` on unions",
        )),
    }
}

pub fn derive_encode_packet(item: TokenStream) -> Result<TokenStream> {
    let mut input = parse2::<DeriveInput>(item)?;

    let Some(packet_id) = find_packet_id_attr(&input.attrs)? else {
        return Err(Error::new(
            input.ident.span(),
            "cannot derive `EncodePacket` without `#[packet_id = ...]` helper attribute",
        ))
    };

    add_trait_bounds(
        &mut input.generics,
        quote!(::valence_protocol::__private::Encode),
    );

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let name = input.ident;

    Ok(quote! {
        impl #impl_generics ::valence_protocol::__private::EncodePacket for #name #ty_generics
        #where_clause
        {
            const PACKET_ID: i32 = #packet_id;

            fn encode_packet(&self, mut w: impl ::std::io::Write) -> ::valence_protocol::__private::Result<()> {
                use ::valence_protocol::__private::{Encode, Context, VarInt};

                VarInt(#packet_id)
                    .encode(&mut w)
                    .context("failed to encode packet ID")?;

                self.encode(w)
            }
        }
    })
}
