use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{parse2, Data, DeriveInput, Error, Fields, LitInt, Result};

use crate::{add_trait_bounds, find_packet_id_attr, pair_variants_with_discriminants};

pub fn derive_encode(item: TokenStream) -> Result<TokenStream> {
    let mut input = parse2::<DeriveInput>(item)?;

    let name = input.ident;
    let string_name = name.to_string();

    let packet_id = find_packet_id_attr(&input.attrs)?
        .into_iter()
        .map(|l| l.to_token_stream())
        .collect::<Vec<_>>();

    match input.data {
        Data::Struct(struct_) => {
            add_trait_bounds(
                &mut input.generics,
                quote!(::valence_protocol::__private::Encode),
            );

            let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

            let encode_fields = match &struct_.fields {
                Fields::Named(fields) => fields
                    .named
                    .iter()
                    .map(|f| {
                        let name = &f.ident.as_ref().unwrap();
                        let ctx = format!("failed to encode field {}", name.to_string());
                        quote! {
                            self.#name.encode(&mut _w).context(#ctx)?;
                        }
                    })
                    .collect(),
                Fields::Unnamed(fields) => (0..fields.unnamed.len())
                    .map(|i| {
                        let lit = LitInt::new(&i.to_string(), Span::call_site());
                        let ctx = format!("failed to encode field {}", lit.to_string());
                        quote! {
                            self.#lit.encode(&mut _w).context(#ctx)?;
                        }
                    })
                    .collect(),
                Fields::Unit => TokenStream::new(),
            };

            let encoded_len_fields = match &struct_.fields {
                Fields::Named(fields) => fields
                    .named
                    .iter()
                    .map(|f| {
                        let name = &f.ident;
                        quote! {
                            + self.#name.encoded_len()
                        }
                    })
                    .collect(),
                Fields::Unnamed(fields) => (0..fields.unnamed.len())
                    .map(|i| {
                        let lit = LitInt::new(&i.to_string(), Span::call_site());
                        quote! {
                            + self.#lit.encoded_len()
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
                        use ::valence_protocol::__private::{Encode, Context, VarInt};

                        #(
                            VarInt(#packet_id)
                                .encode(&mut _w)
                                .context("failed to encode packet ID")?;
                        )*

                        #encode_fields

                        Ok(())
                    }

                    fn encoded_len(&self) -> usize {
                        use ::valence_protocol::__private::{Encode, Context, VarInt};

                        0 #(+ VarInt(#packet_id).encoded_len())* #encoded_len_fields
                    }
                }

                #(
                    #[allow(unused_imports)]
                    impl #impl_generics ::valence_protocol::__private::DerivedPacketEncode for #name #ty_generics
                    #where_clause
                    {
                        const ID: i32 = #packet_id;
                        const NAME: &'static str = #string_name;

                        fn encode_without_id(&self, mut _w: impl ::std::io::Write) -> ::valence_protocol::__private::Result<()> {
                            use ::valence_protocol::__private::{Encode, Context, VarInt};

                            #encode_fields

                            Ok(())
                        }

                        fn encoded_len_without_id(&self) -> usize {
                            use ::valence_protocol::__private::{Encode, Context, VarInt};

                            0 #encoded_len_fields
                        }
                    }
                )*
            })
        }
        Data::Enum(enum_) => {
            add_trait_bounds(
                &mut input.generics,
                quote!(::valence_protocol::__private::Encode),
            );

            let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

            let variants = pair_variants_with_discriminants(enum_.variants.into_iter())?;

            let encode_arms = variants
                .iter()
                .map(|(disc, variant)| {
                    let variant_name = &variant.ident;

                    let disc_ctx = format!(
                        "failed to encode enum discriminant {disc} for variant {}",
                        variant_name.to_string()
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
                                        "failed to encode field {} in variant {}",
                                        name.to_string(),
                                        variant_name.to_string()
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
                                        "failed to encode field {} in variant {}",
                                        name.to_string(),
                                        variant_name.to_string()
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
                            Self::#variant_name => Ok(VarInt(#disc).encode(&mut _w)?),
                        },
                    }
                })
                .collect::<TokenStream>();

            let encoded_len_arms = variants
                .iter()
                .map(|(disc, variant)| {
                    let name = &variant.ident;

                    match &variant.fields {
                        Fields::Named(fields) => {
                            let fields = fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();

                            quote! {
                                Self::#name { #(#fields,)* } => {
                                    VarInt(#disc).encoded_len()

                                    #(+ #fields.encoded_len())*
                                }
                            }
                        }
                        Fields::Unnamed(fields) => {
                            let fields = (0..fields.unnamed.len())
                                .map(|i| Ident::new(&format!("_{i}"), Span::call_site()))
                                .collect::<Vec<_>>();

                            quote! {
                                Self::#name(#(#fields,)*) => {
                                    VarInt(#disc).encoded_len()

                                    #(+ #fields.encoded_len())*
                                }
                            }
                        }
                        Fields::Unit => {
                            quote! {
                                Self::#name => VarInt(#disc).encoded_len(),
                            }
                        }
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

                        #(
                            VarInt(#packet_id)
                                .encode(&mut _w)
                                .context("failed to encode packet ID")?;
                        )*

                        match self {
                            #encode_arms
                            _ => unreachable!(),
                        }
                    }
                    fn encoded_len(&self) -> usize {
                        use ::valence_protocol::__private::{Encode, Context, VarInt};

                        #(VarInt(#packet_id).encoded_len() +)* match self {
                            #encoded_len_arms
                            _ => unreachable!() as usize,
                        }
                    }
                }

                #(
                    #[allow(unused_imports)]
                    impl #impl_generics ::valence_protocol::DerivedPacketEncode for #name #ty_generics
                    #where_clause
                    {
                        const ID: i32 = #packet_id;
                        const NAME: &'static str = #string_name;

                        fn encode_without_id(&self, mut _w: impl ::std::io::Write) -> ::valence_protocol::__private::Result<()> {
                            use ::valence_protocol::__private::{Encode, VarInt, Context};

                            match self {
                                #encode_arms
                                _ => unreachable!(),
                            }
                        }
                        fn encoded_len_without_id(&self) -> usize {
                            use ::valence_protocol::__private::{Encode, Context, VarInt};

                            match self {
                                #encoded_len_arms
                                _ => unreachable!(),
                            }
                        }
                    }
                )*
            })
        }
        Data::Union(u) => Err(Error::new(
            u.union_token.span(),
            "cannot derive `Encode` on unions",
        )),
    }
}
